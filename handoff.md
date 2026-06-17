# Wagner — build kickoff handoff

> Written 2026-06-16 for a FRESH session opened in THIS repo (`~/Development/wagner`).
> This is a deliberately long, zero-context-loss handoff (overrides the usual terse
> handoff format — the engineer asked for super-detail). Read it top to bottom, then
> **your first action is to produce a plan** (see §10). Do not start coding freehand.

---

## 0. Orientation — what just happened

Wagner was just **carved out of a larger monorepo** (`dev-ai-utilities`, a Claude Code
plugin marketplace) into this standalone repo, with git history preserved via
`git subtree split`. The old `platform/` prefix is gone — everything is at the root now.
The build that follows happens HERE, in `~/Development/wagner`.

- **Local-only.** No git remote yet. When you add one, the org uses **GitLab, not GitHub**
  (use `glab`, not `gh`).
- **Green baseline** (verified at carve time): `make verify` → 231 cargo (222 `wagner-edge-host`
  + 9 `wagner-edge-shell`) · clippy clean · typecheck clean · 150 vitest (32 shared + 116
  edge/ui + 2 arch) · vite build · 28 deno (hub). Re-run `make verify` first to confirm.
- **The full prior history** (the ~30-update build of the engine, the 002 edge/hub wedge,
  the desktop shell) is in this repo's git log and is summarized in the `the-construct-initiative`
  memory (Updates 1–32). Skim that memory if you need deep backstory.

## 1. What Wagner is — the four pillars (+ Voice)

Wagner is a distributed engineering platform: run autonomous coding agents and accumulate
what they learn into a shared, living knowledge base. **Edge executes, hub remembers.**

| Pillar | One line | Layer |
|---|---|---|
| **Sessions** | Durable, resumable, concurrent agent runs per project | edge |
| **Vault** | Obsidian-style linked-markdown "second brain"; agent reports land as notes | edge ↔ hub |
| **Graph** | Traversable view of the vault — notes as nodes, links as edges | edge (UI) |
| **Sync** | Real-time, offline-first, multi-writer replication of the vault | edge ↔ hub (P2P) |
| **Voice** (now first-class) | Local JARVIS layer — faster-whisper STT + Kokoro TTS, "speak and it routes" | edge |

Canonical design doc: **`docs/wagner-vision-and-architecture.md`** (read this — it has the
component diagram, reuse-vs-net-new table, the loro+iroh design, the 6-phase roadmap, and
the repo strategy). Product strategy: **`edge/ui/PRODUCT.md`** + **`prd.md`** + **`CONTEXT.md`**.

## 2. Locked decisions (do not relitigate)

- **Session model = durable thread per project, like Claude Code.** Open/create a session
  bound to a folder; add goals + steer over time; close; resume later; many coexist.
- **Multi-session UI = left session rail with live status dots** (running / needs-you / idle / done).
- **Entry screen redesign:** new session = native folder picker + type the first goal.
  **Remove** goal-first framing, the guardrails grid (max-iterations / cost-budget /
  blocked-timeout) and the **Test-command** field — the target project's own
  `CLAUDE.md`/`AGENTS.md` declares how it tests.
- **Build scope = the full vision** (resume + concurrency + vault + sync + voice).
- **Build wedge order:** session-entry redesign → durable resume → concurrency →
  **local Vault + graph first** → **then** loro+iroh distributed sync. **Voice runs in parallel** as a first-class pillar.
- **Distributed sync = loro (Rust CRDT) + iroh** (gossip = realtime deltas, iroh-docs = hub
  catch-up store, iroh-blobs = attachments). Org precedent: Atlas runs loro+iroh; `loro-dev/iroh-loro`
  is a reference impl.
- **Vault knowledge model** borrowed from `ar9av/obsidian-wiki` (a Python skill-pack, NOT a
  plugin) but made **deterministic** (parse links/frontmatter in code, LLM only for semantic
  extraction). Adopt its frontmatter: `summary` (≤200ch, powers cheap retrieval), typed
  `relationships: [{target,type}]` (extends/uses/implements/contradicts/derived_from/replaces/related_to),
  `tier`, `lifecycle`, `provenance`; tiered retrieval; `_staging/` human-approval gate.
  Its weakness = exactly our need: it has **zero multi-user sync** (git only) → that's what loro+iroh adds.
- **Repo:** standalone (done). Skills are **not** submodule'd — Wagner reads skills from the
  target repo + global `~/.claude` at runtime; bundle a default set later only if wanted.
- **All real work goes through plan + TDD.** The engine changes below are a genuine refactor.

## 3. Reuse inventory — what's already in THIS repo

- **Vault kernel EXISTS.** `edge/host/src/memory.rs` — `MemoryStore` over embedded SurrealDB
  (SurrealKV) + BM25 (`wagner_en`) index, and it **already dual-writes markdown** to
  `.wagner/memory/<uid>.md` with YAML frontmatter (`write_markdown_projection`, ~line 206).
  Schema carries `user_id`/`project_id` on every row (designed for a central server). This is
  the seed of the Vault — extend it with wikilinks + typed relationships + backlinks.
- **iroh transport seam EXISTS.** `edge/host/src/remote/` — `arm.rs`, `session.rs`, `endpoint.rs`,
  `control.rs`, `devcontext.rs`, `observability.rs`, `mod.rs`. ADR-0003 (`docs/adr/`) selects iroh
  (QUIC, Ed25519, NAT traversal, org relay). The loro layer rides on this.
- **Run persistence EXISTS (resume is ~80% built).** `edge/host/src/state/store.rs` persists each
  `Run` to `{app_data}/runs/{id}/state.json` (validate→temp→fsync→rename) every iteration;
  `state::load()` exists; `RunStatus::Paused` is a variant in `edge/host/src/state/run.rs`. **Missing:
  a `resume_run` + `list_runs` IPC command** that calls `load()` and re-enters the loop.
- **Hub EXISTS** (`hub/`, Deno + Hono): OIDC + ephemeral discovery today. ADR-0001 already names
  it "the always-on peer that remembers" — it becomes the vault sync peer. Article IX limits the hub
  to **metadata + curated learnings** (no raw transcripts/diffs) — vault = curated, fits.
- **Live desktop app EXISTS.** `edge/shell/` (Tauri) + `edge/ui/` (fresh React 19 dark ops console).
  `make edge` builds the frontend + launches it (`--features custom-protocol` makes it serve the
  embedded bundle — without it you get a white screen; that bug was fixed this session).

### NOT in this repo (corrected — these were in the retired `apps/wagner`, history-only in dev-ai-utilities)
- **React Flow is NOT installed here.** The graph view needs `@xyflow/react` added to `edge/ui`.
  A reference (the old WorkflowBuilder DAG) exists only in `dev-ai-utilities` git history at
  `apps/wagner/src/ui/WorkflowBuilder.tsx`.
- **The old PixiJS "districts floor"** (if you ever want a high-density WebGL graph) is likewise
  history-only in dev-ai-utilities (`apps/wagner/.backups/`).

## 4. The engine changes for Sessions (the refactor, with file pointers)

Current reality (single-run): `edge/shell/src/commands.rs` holds `RunManager { current: Mutex<Option<RunControl>> }`
(a single slot; `start_run` overwrites it). UI mirror: `edge/ui/store/reducer.ts` `applyRun` sets a single
`state.run`. Events (`wagner://event|run|transmission`, emitted from `commands.rs`) are NOT keyed by run_id.

To support durable + concurrent sessions:
1. **`RunManager`: `Mutex<Option<RunControl>>` → `Mutex<HashMap<RunId, RunControl>>`** (+ `abort` targets one id).
2. **Add IPC commands:** `resume_run(run_id)` (calls `state::load`, rebuilds the pool, re-enters the loop),
   `list_runs()` (reads `{app_data}/runs/*/state.json`), `get_run(id)`.
3. **Key events by run_id** — either `wagner://run/{id}` channels or a `run_id` discriminator in every payload;
   update `commands.rs` emit sites + `edge/ui/transport/ipc.ts` + the reducer.
4. **Reducer:** `state.run: RunSnapshot | null` → `state.runs: Record<RunId, RunSnapshot>`; the session rail
   reads the map. Keep the reducer pure (replay-safe) — there are existing replay tests; don't break them.
5. **Goal model:** today goal is one-shot baked at `Run::new`; `steer` augments planning context but doesn't
   add goals. The "add a goal in a session" UX needs either a goal queue on `Run` or a session entity above it
   — design this in the plan.

Known latent bug to fix while here: `edge/ui/transport/ipc.ts` `send` invokes `wagner_${kind}` (e.g.
`wagner_steer`) but the real commands are `steer`/`abort`/`answer_transmission` (the desktop UI sidesteps it
by invoking real names; fix before the P2P control path).

## 5. The distributed Vault design (when you reach the sync phase)
loro: **one `LoroDoc` per note** (bounded merge), one shared **`LoroTree`** for folder hierarchy (UUID node
ids, paths derived — movable tree handles concurrent rename/move). iroh-gossip = realtime deltas per open note;
iroh-docs = hub catch-up store keyed by note UUID; iroh-blobs = attachments. Wikilink integrity under concurrent
rename via a UUID↔display-name `LoroMap` index + projector rewrite. **The one hard/risky component = the
file↔CRDT projector** (a `notify-rs` watcher diffs Obsidian's on-disk edits into loro ops; merges project back
to `.md` only when no live local edit is in flight — needs file-lock + CAS-on-last-projected-hash). Budget real
test time for it. Full detail + risks in `docs/wagner-vision-and-architecture.md` §5/§9.

## 6. Build & run commands
- `make verify` — full gate (run first).
- `make edge` — build frontend + launch the native desktop app.
- `make edge-build` — rebuild just the frontend bundle (if the shell serves stale embedded assets).
- `make edge-ui` — headless Playwright UI smoke (browser-level; can't drive the native shell).
- Individual gates: `make cargo`, `make clippy`, `make shell`, `make ts`, `make typecheck`, `make arch`, `make hub`.
- Toolchain: `rust-toolchain.toml` pins 1.91.1 (iroh 1.0 needs >1.90).

## 7. Voice (now first-class, parallel pillar)
Local pipeline: mic → **faster-whisper** (STT, CTranslate2) → route (regex / local model / Haiku) →
**Kokoro** (82M open-weight TTS) speaks back. Ears + mouth stay 100% local. No code exists yet — design a clean
seam (a `voice/` module on the edge) in its own plan. It's parallel to, not blocking, the core loop.

## 8. Git state (this repo)
- Branch: `main`. Remote: none yet (use **GitLab/`glab`** when ready).
- History: 9 commits, preserved from the carve (`d636c96` wedge scaffold → … → `45ad0b4` custom-protocol fix →
  `d7b7d7a` vision/PRODUCT docs → `50ff5a2` Makefile/gitignore → `82990fc` ignore IDE dirs).
- Working tree: clean (plus this `handoff.md`).

## 9. Risks & deferred
- **Projector race** (file watcher ↔ CRDT) — the highest-risk component; no off-the-shelf solution for Obsidian.
- **iroh-docs LWW vs loro ordering** at the hub — make the hub's in-memory LoroDoc the authoritative merge.
- **loro has no awareness in Rust** — build a thin presence layer over iroh-gossip behind a trait.
- Optional polish carried over: `operative_id`→`agent` rename (~29 files), `CostBudget` f64→microdollars,
  `run_test` async, driver mpsc bounding. Not blockers.

## 10. YOUR FIRST ACTION
Do **not** start editing. Open with a **plan** for the first wedge:
**(session-entry redesign) + (durable resume + concurrent sessions engine refactor)**.
Use `/write-plan` (or `/spec` if you judge the interpretation surface wide enough — this touches the engine
contract, so `/spec` is defensible). The plan must:
- Cover the §4 engine changes (RunManager map, run_id-keyed events, reducer `run→runs`, resume/list IPC, goal model).
- Cover the §2 entry-screen redesign (folder picker, drop guardrails grid + test field, session rail).
- Be TDD-ordered (there are existing cargo + vitest + replay tests — extend, don't break).
Then `/tdd` → `/execute-plan`. After this wedge: local Vault v1 + graph (add `@xyflow/react`), then loro+iroh sync.
Voice gets its own parallel plan.

Pointers: `docs/wagner-vision-and-architecture.md` (architecture/roadmap), `edge/ui/PRODUCT.md` (product register),
`prd.md` + `CONTEXT.md` (platform PRD + glossary), `docs/adr/` (ADR-0001 hub/SurrealDB/Deno, 0002 OIDC, 0003 iroh).
Memory: `the-construct-initiative` (Updates 1–32 — Update 32 records this carve).
