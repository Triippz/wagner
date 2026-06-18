# Wagner — vision & architecture

> Status: living architecture note. `../VISION.md` is the product source of
> truth; this document explains how the current edge/hub architecture supports
> that vision.
>
> **v2 reframe (2026-06-17):** the five-pillar v1 (Sessions, Vault, Graph, Sync,
> Voice) is built. v2 widens the product from an *engineering* platform to a
> **local-first personal OS** — a common operating picture over daily work:
> agents, workflows, knowledge, search, media/artifact generation, connectors,
> projects, and tools. Coding is one heavy workspace, not the product boundary.
> See §12.

## 1. What Wagner is

Wagner is a local-first, cloud-connected personal OS for daily work. It is the
operator's command surface for agents, workflows, knowledge, search, connectors,
and dedicated workspaces. Coding and development are central because they are the
first heavy workspace, but the product is not a coding IDE: it should also handle
research, news briefings, image/artifact generation, productivity automation,
and general search.

The architecture still has two layers, one product:

- **Edge executes.** A native desktop app runs agent **sessions** and local
  workflows on the operator's machine. For coding, it can drive Codex, Claude
  Code, and future harnesses against real repos. For broader work, it should
  dispatch agents, deterministic workflows, connector calls, search/research
  tasks, and artifact generation without requiring a cloud round trip.
- **Hub remembers and coordinates.** An always-on peer syncs the shared **Vault**
  and coordination metadata across devices or teams. The vault is an
  Obsidian-style second brain of linked markdown notes and typed relationships
  that sessions, workflows, and connectors read from and write back to.

The model borrows from JARVIS, the Karpathy "LLM wiki" pattern, and Palantir
Maven-like smart systems in the sense of one operating picture over typed
knowledge and coordinated action. It is not an OSINT product. It is a personal
work OS whose vault can remain private, sync across the operator's devices, or be
shared with a team.

## 2. The four pillars

| Pillar | One line | Layer |
|---|---|---|
| **Sessions** | Durable, resumable, concurrent agent runs per project | edge |
| **Vault** | Linked-markdown second brain; agent reports land here as notes | edge ↔ hub |
| **Graph** | A traversable view of the vault — notes as nodes, links as edges | edge (UI) |
| **Sync** | Real-time, offline-first, multi-writer replication of the vault | edge ↔ hub (P2P) |

Designed-for-but-later has moved into the core vision: **Voice** is first-class
and text remains equal; the engine should be **model/harness-agnostic** (Codex,
Claude Code, Claude Agent SDK, OpenAI/other providers, local models); and the
system should remain extensible enough to reskin, self-host, and share per
operator/client/team.

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

v1 phases 0–6 + voice + hub are **shipped**. The next arc is **v2 — the command
center** (§12); near-term items there are voice audio I/O, then user-authored
agents via the Claude Agent SDK.

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

## 12. Wagner v2 — the personal OS

v1 made Wagner a distributed *engineering* platform. v2 keeps that engine but
reframes the product around a single idea the operator stated directly: **a
common operating picture of daily work** — agents, deterministic workflows,
knowledge, search, news, media/artifact generation, projects, connectors, and the
whole productivity stack — run from one surface. "Command center" means an
operating picture for personal work, not a military or OSINT product.

The pillars don't change; their *scope* widens. The Vault already holds
knowledge + learnings; the Graph already renders it. v2 adds first-class
**Agents**, **Workflows**, **Connectors**, **Search/Research**, and
**Workspaces**, and promotes the HUD (§6) from "session view" to the operating
picture across all of them.

### A. User-authored agents (Claude Agent SDK) — near-term

Today agents are a fixed oracle/operative **roster** run by spawning the
`claude`/`codex` CLI as a child and parsing stream-json (`edge/host/src/cli/`,
`orchestrator/roster.rs`). v2 makes **agents a thing the operator defines**:

- An **agent definition** = `{ name, system prompt, model, tools/harness,
  allowed dirs }`, stored as a Vault note (dogfood the knowledge model — agents
  are knowledge too) so they version, link, and sync like everything else.
- Run them **locally** via the **Claude Agent SDK** rather than only the CLI —
  programmatic control over system prompt, model, and tool surface per agent.
  The existing CLI driver stays as one backend behind the `AgentPool` trait; the
  SDK becomes a second backend (model-agnostic seam already planned in §2).
- **Custom system prompts / models / harnesses per agent** is the headline
  requirement — the agent definition is exactly that knob.

### B. Connectors — the productivity stack (later, the big widening)

Wagner reaches beyond coding: **email, Slack, Discord, calendar, docs** — run
the operator's productivity tools from the same surface. The natural seam is
**MCP** (many of these connectors already exist as MCP servers in the operator's
environment). Agents (A) act over Connectors; their outputs land in the Vault as
notes; the Graph/HUD shows the operating picture. This is the step that turns
Wagner from "coding tool" into "personal agentic OS" — and the largest scope, so
it lands last and incrementally (one connector at a time).

### C. Cloud agents / agent registry — stretch

Extend the **Hub** (already the always-on peer + vault sync) to **store agent
definitions + config** so others can run them, and — stretch — to **run agents
in the cloud**, not just on the operator's machine. Article IX still binds:
curated agent configs + metadata may sync; raw transcripts stay edge-local.
Depends on A (definitions exist) and reuses the hub sync stack from Phase 6.

### v2 roadmap

| Phase | Deliverable | Depends on |
|---|---|---|
| **v2.0** | **Voice audio I/O** — mic→STT, TTS→speaker loop (carried from v1 "later"; see handoff) | voice engine (done) |
| **v2.1** | **Agent definitions** — author/store agents as Vault notes (name, system prompt, model, tools, dirs) + UI | Vault (done) |
| **v2.2** | **Claude Agent SDK backend** behind `AgentPool` — run authored agents locally with per-agent prompt/model/harness | v2.1 |
| **v2.3** | **Connectors v1** — first MCP-backed non-coding tool (pick one: email *or* Slack), outputs → Vault notes | v2.2 |
| **v2.4** | **HUD = operating picture** — one surface over sessions + agents + vault + connectors | v2.1–v2.3 |
| **later** | More connectors (Discord, calendar, docs…), one at a time | v2.3 |
| **stretch** | **Cloud agents / registry** — hub stores agent configs for others; run-in-cloud | v2.1 + hub (done) |

Open v2 decisions: (1) Agent SDK = TypeScript or Python SDK, and where it runs
relative to the Tauri-free host (mirror the voice-sidecar lifecycle?). (2) First
connector to prove the MCP seam. (3) Does the SDK backend *replace* the CLI
driver for authored agents, or coexist (recommended: coexist).

## 13. v2 backlog — operator additions (2026-06-17)

Captured as direction; not yet sequenced into phases. Several are product-shaping
(reshape v2.4's HUD and the engine), not single features.

1. **Obsidian-core, local-first, optional cloud sync.** Make the Obsidian vault
   *the* knowledge layer, not a side feature: local-first by default, cloud sync
   opt-in. Wagner already writes Obsidian-compatible `.md` + `[[wikilinks]]` via
   the projector — this elevates it to a primary surface (embedded editor +
   graph). Obsidian the app is closed-source; the *format* is open and is what we
   target. OSS embeddable editors to evaluate: Logseq, SilverBullet, Foam.
   Refs: SurrealDB-as-KG article (operator-supplied), `colleague-skill#125`.

2. **SurrealDB as the distributed knowledge-graph backend.** Lean on SurrealDB
   (already the vault's store + BM25 index) for the distributed graph — live
   queries, semantic/contextual storage, possibly TiKV-backed distribution.
   **Open architectural fork vs. the shipped loro+iroh CRDT sync** (§5): is
   SurrealDB a *complement* (semantic index/query) or a *replacement* for the
   CRDT transport? Do not silently swap — decide deliberately. Ref: operator's
   SurrealDB-KG article.

3. **General productivity tool, not a coding IDE.** Stop optimizing the UI for
   coding/dev specifically. Coding is one workspace among many (notes, agents,
   connectors, projects). This directly shapes v2.4 (HUD = operating picture) and
   item 8 (customizable UI) — the shell must not assume "a repo + a run."

4. **Self-hostable multi-tenant auth — not just SSO/SAML.** Anyone can host the
   hub in the cloud, create users via **username/password** (or similar), and
   invite their own people. The hub already has OIDC; this adds a
   self-host-friendly local auth + invite + tenant model. Article IX
   (metadata/curated-only sync) still binds per tenant.

5. **Deterministic + multi-provider headless workflows.** Beyond conversational
   agents: run **deterministic workflows** headlessly. Use the Agent SDK for
   Claude *and* OpenAI/others (multi-provider, model-agnostic — extends §2's
   model-agnostic engine + the `AgentPool` trait). A workflow engine — Temporal
   is the heavy reference; prefer something lightweight (or in-process durable
   steps) unless durability demands more. This generalizes v2.2.

6. **Easy MCP server management.** First-class UI to add/configure MCP servers
   and make them trivially usable by agents (item 5) and connectors (§12.B). MCP
   is the connector seam; this is its control panel.

7. **Plugin system (code-based) + plugin registry.** Plugins authored in code; a
   **central repository** for community plugins plus support for **private**
   plugins. Ties to item 8 (UI extension points) and item 6 (MCP/agents as plugin
   surface). Big — needs a plugin API/contract and a trust/sandboxing model.

8. **Fully customizable UI (VSCode-style).** Make as much of the UI configurable
   as possible — layout, panels, theme, keybindings — extensible via plugins
   (item 7). This and item 3 push the shell toward a generic, dockable,
   theme-able workbench rather than a fixed coding console.

9. **Pluggable harnesses + models — agentic coding beyond Claude/Codex.** Treat
   the *harness* (the agentic coding tool) as swappable, and the *model* behind it
   as independent. The seam already exists: agents run behind the `AgentPool`
   trait with a per-harness event mapper (`events/map_claude.rs`,
   `events/map_codex.rs`) — each new harness = a driver invocation + a mapper.
   Targets:
   - **Cursor (incl. its CLI)** and **OpenCode** as first-class harness backends,
     alongside the existing `claude`/`codex`.
   - **Self-hosted models** and **alternative cloud models** (e.g. **GLM 5.2**)
     selectable per agent — extends the model-agnostic engine (§2) and the
     per-agent model knob (§12.A, §13.5).
   - **Skill portability:** the same skill packs (SKILL.md) the operator uses in
     Claude/Codex must work across harnesses — a harness-neutral skill-loading
     contract, not per-harness rewrites. This is the hard part (each harness
     discovers/loads skills differently).
   Relationship: §13.5 = programmatic Agent-SDK workflows (multi-provider via
   SDK); this item = full third-party agentic harnesses as drop-in backends. Both
   ride the same `AgentPool` seam.

**Priority note (operator, 2026-06-17):** *before* any of §13, get the existing
v1 product **working as intended** end-to-end (not just CI-green). That is the
immediate focus; §13 is the queue behind it.
