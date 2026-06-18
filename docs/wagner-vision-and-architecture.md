# Wagner — vision & architecture

> Status: proposal (2026-06-16). Supersedes the "Wagner Edge run console" framing.
> Wagner is the product; the desktop app is one layer of it.

## 1. What Wagner is

Wagner is a distributed engineering platform for running autonomous coding agents
and accumulating what they learn into a shared, living knowledge base. Two layers,
one product:

- **Edge executes.** A native desktop app runs agent **sessions** against the
  operator's repos — the oracle plans, a roster of operatives execute. Runs are
  durable and survive closing the window.
- **Hub remembers.** An always-on peer holds the shared **Vault** — an
  Obsidian-style second brain of linked markdown notes that every session reads
  from and writes back to, synced in real time across teammates.

The model is borrowed from JARVIS / the Karpathy "LLM wiki" pattern, but made
**distributed**: the vault is not one person's local folder, it is a
conflict-free replicated knowledge graph shared across the team.

## 2. The four pillars

| Pillar | One line | Layer |
|---|---|---|
| **Sessions** | Durable, resumable, concurrent agent runs per project | edge |
| **Vault** | Linked-markdown second brain; agent reports land here as notes | edge ↔ hub |
| **Graph** | A traversable view of the vault — notes as nodes, links as edges | edge (UI) |
| **Sync** | Real-time, offline-first, multi-writer replication of the vault | edge ↔ hub (P2P) |

Designed-for-but-later: **Voice** (a local JARVIS layer — faster-whisper STT +
Kokoro TTS), **model-agnostic** engine (swap Claude/Codex/local models), and a
**forkable skeleton** (reskin per operator/client/team).

## 3. Grounding — what already exists vs what is net-new

This is the key finding of the recon: Wagner is **not greenfield** on most of this.

### Reuse inventory (exists today)

- **Vault kernel.** `platform/edge/host/src/memory.rs` already dual-persists every
  learning as `.wagner/memory/<uid>.md` with YAML frontmatter ("Serena-style"),
  backed by SurrealDB + a BM25 (`wagner_en`) full-text index. Schema carries
  `user_id`/`project_id` on every row *specifically so the same schema serves a
  central server by filtering, not rewriting*. This is a primitive markdown vault
  already.
- **Graph view base.** `@xyflow/react` (React Flow) is already installed and
  powering the WorkflowBuilder DAG — node/edge binding, zoom, pan, minimap for
  free. The old PixiJS "districts floor" survives in `apps/wagner/.backups/` if a
  high-density animated WebGL graph is ever wanted.
- **P2P transport.** `platform/edge/host/src/remote/` (arm/session/endpoint/
  control) + ADR-0003 selects **iroh** (QUIC, Ed25519 identity, NAT traversal,
  org-run zero-knowledge relay). Hub discovery registry is live.
- **Hub.** `platform/hub/` (Deno + Hono) — OIDC + ephemeral discovery today;
  ADR-0001 already names it "the always-on peer that remembers." Article IX
  constrains it to **metadata + curated learnings only** (never raw code/diffs) —
  this directly shapes what may sync to the hub.

### Net-new

- The **loro** CRDT layer + the file↔CRDT **projector** (the one genuinely hard
  part — see §5).
- Wikilinks + typed relationships + backlinks over the existing notes.
- The graph-view React component + the Vault browser UI.
- Hub vault-sync routes + durable store.
- The session-model engine changes (single-run → durable/concurrent — see §7).

## 4. The Vault — knowledge model (borrowed from obsidian-wiki, made deterministic)

`obsidian-wiki` is a Python skill pack, not a plugin; its reusable gold is its
**knowledge model**, which we adopt and harden:

- **Frontmatter schema** per note: `title`, `summary` (≤200 chars — powers cheap
  tiered retrieval), `tags`, `tier` (core/supporting/peripheral), `lifecycle`
  (draft→reviewed→verified→disputed→archived), `provenance` (extracted/inferred/
  ambiguous), and typed `relationships: [{target, type}]` where type ∈
  {extends, uses, implements, contradicts, derived_from, replaces, related_to}.
- **Tiered retrieval**: `summary` grep → section grep → full read → BFS over
  `relationships`. Keeps query cost ~constant as the vault grows.
- **The vault is a compiled artifact, not a cache.** When a session produces
  insight, the agent *updates affected notes and links them*, it does not dump a
  transcript. A `_staging/` human-approval gate keeps it trustworthy.
- **Build differently from obsidian-wiki:** linking/backlink extraction is
  **deterministic code** (parse frontmatter + `[[wikilinks]]` with a real
  parser), not an LLM-reads-SKILL.md step. The LLM is used only for semantic
  extraction (what entities/claims are in this doc); the graph writes are
  mechanical. This is the project's "deterministic over probabilistic" principle.

Notes keep **UUID identity**; the human-readable `[[Display Name]]` in the `.md`
is projected from a UUID→name index so renames don't break links (§5).

## 5. Distributed sync — loro + iroh (the hard part)

Recommendation is unambiguous and has org precedent (Atlas already runs loro+iroh;
`loro-dev/iroh-loro` is a reference impl).

```
        EDGE (Tauri app)                         HUB (24/7 peer)
 ┌───────────────────────────────┐        ┌────────────────────────┐
 │ Obsidian / editor             │        │ iroh-docs store        │
 │   .md files on disk           │        │  (latest snapshot/note)│
 │        ▲          │           │        │ iroh-blobs (attachments)│
 │        │ projector│ (notify)  │        │ "always has latest"    │
 │        ▼          ▼           │        └───────────▲────────────┘
 │ VaultCrdt (loro)              │                    │ QUIC (iroh)
 │   LoroDoc per note (LoroText) │   gossip deltas    │
 │   LoroTree (folder hierarchy) │◄───────────────────┼────► PEER (teammate)
 │   LoroMap (uuid→name index)   │                    │      same stack
 │ SyncAdapter (iroh)            │────────────────────┘
 │   gossip=realtime deltas      │
 │   docs=catch-up after offline │
 └───────────────────────────────┘
```

Key decisions:
- **One `LoroDoc` per note** (bounded merge scope, per-note time-travel), a single
  shared **`LoroTree`** for the folder hierarchy (movable-tree handles concurrent
  renames/moves; nodes are UUIDs, paths are derived).
- **iroh-gossip** delivers realtime deltas (50–500 B/keystroke-batch) per open
  note; **iroh-docs** is the hub's durable catch-up store keyed by note UUID;
  **iroh-blobs** for attachments. Identity = iroh Ed25519 peer keys; the iroh-docs
  namespace secret governs write capability. **Presence/awareness** rides
  iroh-gossip (loro's Rust crate has no awareness).
- **The projector (highest risk).** A `notify-rs` watcher diffs external edits
  (Obsidian writing a file) into loro `LoroText` ops; incoming merges are exported
  and atomically written back **only when no live local edit is in flight**
  (debounced, last-projected hash as a CAS guard). This bidirectional bridge is
  the one component with a real race window and must be built and heavily tested.
  See §9 risks.
- **Wikilink integrity under concurrent rename:** UUID is canonical; a `LoroMap`
  link index maps UUID↔display-name; the projector rewrites stale `[[names]]` on
  the next pass and preserves old names as Obsidian `aliases` for a grace period.

**Hub boundary (Article IX):** only curated notes + metadata sync to the hub —
never raw agent transcripts/diffs. The vault is curated knowledge by construction,
so it fits; raw run logs stay edge-local.

## 6. Graph view & HUD

- **Graph view**: React Flow over the vault — notes as nodes (color by `tier`/
  lifecycle), typed `relationships` as edges, click-to-open, neighborhood
  highlight, filter by tag/lifecycle. React Flow is already installed.
- **HUD** (the JARVIS "V.A.U.L.T." one-screen idea): a single ops surface tying
  session activity + vault graph + (later) voice. We resist the literal sci-fi
  particle HUD — the product register is "the tool disappears"; we take the
  *integration* idea (one surface), not the decoration.

## 7. Sessions (workstream 1 — the redesign already in flight)

The entry-screen redesign decided earlier folds in here:
- New session = **pick a folder (native dialog) + type the first goal.** No
  guardrails grid, no test-command field (the project's own `CLAUDE.md`/`AGENTS.md`
  declares how it tests).
- **Durable + resumable + concurrent**, surfaced as a **left session rail** with
  live status. Engine reality (from recon): runs already persist to
  `{app_data}/runs/{id}/state.json` + `state::load()` + `RunStatus::Paused` exist —
  resume is ~80% built. Concurrency needs `RunManager: Option→HashMap`,
  run_id-keyed events, and reducer `run→runs`.

## 8. Roadmap

| Phase | Deliverable | Mostly |
|---|---|---|
| **0** | Session entry redesign: folder picker, drop guardrails grid, session-first copy | frontend |
| **1** | Durable resume + session rail (single live → list + reopen) | engine (small) |
| **2** | Concurrent sessions (RunManager map, keyed events, reducer Record) | engine |
| **3** | Local Vault v1: wikilinks + typed links + backlinks over `.wagner/memory`, deterministic linker | engine |
| **4** | Graph view + Vault browser (React Flow) | frontend |
| **5** | Distributed sync v1: loro per-note + iroh gossip/docs + the projector | engine (hard) |
| **6** | Hub vault store + multi-teammate sync + presence | edge+hub |
| **later** | Voice (faster-whisper + Kokoro), model-agnostic engine, forkable skeleton | both |

## 9. Top risks

1. **The projector race** (file watcher ↔ CRDT, both directions). Narrow but real
   window where an Obsidian write during a remote merge corrupts an op. Mitigate
   with file locking + CAS on the last-projected hash + careful Tokio design;
   budget real test time. No off-the-shelf solution for the Obsidian case.
2. **iroh-docs LWW vs loro ordering** at the hub — solved by making the hub's
   in-memory LoroDoc the authoritative merge and iroh-docs only the snapshot store,
   with a sequential per-note import task.
3. **loro awareness gap in Rust** — build a thin `PresenceBroadcaster` over
   iroh-gossip behind a trait so it's swappable; failures degrade to "no presence."
4. **Scope.** Six phases across two layers, a Rust engine, a CRDT, and a P2P stack.
   This needs its own repo, its own CI, and disciplined phasing (see §10).

## 10. Repo strategy — graduate Wagner out of dev-ai-utilities

> **STATUS: DONE.** Wagner now lives in its own standalone `wagner/` repository
> (the monorepo layout below). The original recommendation is kept for context.

Recommendation: **yes, move Wagner to its own repository.** It has crossed from
"a utility inside a Claude Code plugin marketplace" to a shipping distributed
product with a fundamentally different lifecycle (signed Tauri release builds,
cargo + npm + Deno workspaces, iroh/loro deps, its own CI/issues/versioning).

Proposed shape — the "one monorepo, two layers" the initiative already describes:

```
wagner/
  edge/          # Rust engine (host) + Tauri shell + React UI
  hub/           # Deno + Hono always-on peer
  shared/        # schemas, the vault CRDT crate, the event/reducer contract
  skills/        # consumed from dev-ai-utilities (submodule or vendored)
  docs/ adr/
```

Wagner **consumes** the dev-ai-utilities skill/agent library (that library *is*
the JARVIS "skill architecture" — folders of SKILL.md the agents load on demand),
so the relationship is a dependency (git submodule or a published skills package),
not a monorepo coupling. This cleanly resolves the existing `apps/wagner` ↔
`platform/edge` duplication noted in the last handoff: the new repo keeps one
engine tree.

Sequence: lock this architecture → carve the repo as a deliberate migration →
collapse the duplicate tree in the move. Not a now-this-second cutover.

## 11. Open decisions (for the operator)

1. **Repo**: graduate to own repo now vs. after the session redesign ships.
2. **Build wedge**: which pillar first after sessions — local Vault+graph
   (single-machine, immediately useful) before distributed sync, or go straight
   for the distributed CRDT?
3. **Voice**: design-for-now-build-later (recommended) vs. in-scope now.
