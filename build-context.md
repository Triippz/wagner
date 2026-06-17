# Wagner — autonomous build context (live resume anchor)

> **Single source of truth for an unattended, autonomous build.**
> Fresh agent (post-compaction or new session): read this top to bottom, then
> `memory/000-index.md`, then resume at **§ Current position**. Keep this file
> current — update it at the end of every step.
>
> (Not to be confused with `CONTEXT.md` — that's the domain glossary.)

Last updated: 2026-06-17 (autonomous build kickoff)

## Mission

Build the **entire Wagner product to 100%** autonomously, no stopping — plus
local-dev/devex setup, full E2E testing, and Docker for the server (hub).
Commit each green step (to `feat/autonomous-build`; see rule 1). No questions
unless a hard blocker forces it. Decisions come from the plans + sensible
defaults, recorded in `memory/`.

### Parallel lanes — BOTH INTEGRATED ✓
- `lane/voice` — Voice pillar (Stt/Tts traits + FakeStt/FakeTts + VoiceRouter +
  VoicePipeline + 9 integration tests). Merged. `edge/host/src/voice/**`.
- `lane/devex` — `hub/Dockerfile` + `docker-compose.yml` + `hub/.dockerignore`,
  hub E2E (`hub/tests/e2e/hub_server.test.ts`, `make hub-e2e`, 7 tests),
  `docs/development.md`, Makefile `dev-setup`/`docker-hub`. Merged.

Wagner = a distributed engineering platform: run autonomous coding agents and
accumulate what they learn into a shared, living knowledge base. **Edge executes,
hub remembers.** Five pillars: Sessions, Vault, Graph, Sync, Voice.

Authoritative design: `docs/wagner-vision-and-architecture.md`. Kickoff:
`handoff.md`. Domain language: `CONTEXT.md`. Locked decisions: `handoff.md` §2 +
`memory/`.

## Operating rules (the autonomy contract)

1. **Commit each green step** to **`feat/autonomous-build`** (a harness hook
   blocks direct `main` commits unattended — see `memory/005`; operator
   fast-forwards `main` on waking). One atomic commit per plan step, ONLY when
   `make verify` is green. **Never commit red.** No pushes (no remote). Commit
   messages: no "Claude"/AI references; stage explicit paths (no `git add -A`).
2. **Backups before large edits** → `.backups/` (gitignored). Global convention.
3. **TDD always.** Red test → implement → green → refactor → commit.
4. **Acceptance-gated.** `make verify` = fast gate (unit+integration+build).
   `make accept` = full gate (verify + UI journey). `verify` per step, `accept`
   at phase boundaries.
5. **Record every non-obvious decision** as `memory/NNN-*.md` (frontmatter +
   wikilinks, Obsidian/Karpathy style — we dogfood the Vault model). Index it in
   `memory/000-index.md`.
6. **Update this file** (§ Current position + roadmap checkboxes) every step.
7. **Parallelize safely.** Main loop owns `main` + the critical path (sequential,
   gated). Subagents do isolated-file work (new specs/docs/tests) or read-only
   recon/review. Cheap models (sonnet) for mechanical/recon; opus for hard build.
   Never let two writers touch the same file on `main`.
8. **Stop on a true blocker** — write it to § Blockers, leave a clean tree, do
   not guess destructively.

## Roadmap (the whole product — check off as shipped)

Plans live in `specs/<NNN>-<slug>/plan.md`. Each gets acceptance tests first.

- [x] **Phase A — Foundation**: context layer (this file + memory/) ✓ +
      `make accept` gate (verify + UI journey) ✓ + `specs/003-*/acceptance.md`
      (Given/When/Then definition of done) ✓.
- [x] **Phase 0–2 — Sessions** (`specs/003-durable-concurrent-sessions`): DONE.
      All 10 steps committed; `make accept` green. Entry redesign (folder picker
      + goal), Run-as-session fields, RunManager map, run_id-keyed reducer +
      selectors, resume/list/get/add_goal IPC, ipc.ts fix, session rail.
- [x] **Phase 3 — Vault v1** (`specs/004-vault-v1`): COMPLETE (8 steps). Frontmatter
      scalars; deterministic wikilink parser (`vault/linker.rs`); name-index/
      wikilink/backlink/relationship tables + methods; `save_note` unified write
      path; tiered_query + `related_by_bfs`; `_staging/` approval gate;
      vault_summary/approve_staging/list_staging IPC + bridge.
      NOTE: SurrealDB 2.x rejects enums/Option/nested → persisted fields are plain
      scalars; typed relationships live in a table, not on MemoryRecord.
- [x] **Voice** (`specs/007-voice`): COMPLETE (merged from lane/voice).
- [x] **Devex/Docker/E2E**: hub Dockerfile + compose, `make hub-e2e` (7 tests),
      `docs/development.md`, `make dev-setup`/`docker-hub` (merged from lane/devex).
- [ ] **Phase 4 — Graph + Vault browser** (`specs/005-graph-view`): React Flow
      over the vault (`@xyflow/react` to add). Depends on Phase 3.
- [ ] **Phase 5 — Distributed sync v1** (`specs/006-sync`): loro per-note + iroh
      gossip/docs + the file↔CRDT projector (highest risk). Depends on Phase 3.
- [ ] **Phase 6 — Hub vault store + multi-teammate sync + presence**.
- [ ] **Voice** (`specs/007-voice`): faster-whisper STT + Kokoro TTS, `voice/`
      seam. Fully parallel (handoff §7).

## Current position

**Plan 003 (Sessions, Phases 0–2) COMPLETE** — 10 steps committed on
`feat/autonomous-build`, `make accept` green. Foundation (Phase A) + Plans 004
(Vault) & 007 (Voice) authored.

**DONE:** Foundation, Sessions (003), Vault v1 (004), Voice (007), Devex/Docker/
hub-E2E. All merged into `feat/autonomous-build`; `make verify` + `make hub-e2e`
green.

**Next, in roadmap order:**
1. **Phase 4 — Graph view + Vault browser** (`specs/005-graph-view`, to author):
   add `@xyflow/react` to `edge/ui`; a graph IPC that returns nodes/edges from the
   vault (uses `related_by_bfs` + relationship/backlink tables — may need a new
   `vault_graph(project_dir)` command); React Flow component (nodes=notes colored
   by tier/lifecycle, edges=typed relationships); a vault browser panel + a
   staging-approval UI (uses list_staging/approve_staging). UI-heavy → parallel-able.
2. **Phase 5 — Distributed sync v1** (`specs/006-sync`, to author): loro per-note
   + iroh gossip/docs + the file↔CRDT projector (HIGHEST RISK — `docs/wagner-
   vision-and-architecture.md` §5/§9). Do in the main loop with heavy tests; ride
   the existing `edge/host/src/remote/` iroh seam. Add loro + iroh deps.
3. **Phase 6 — Hub vault store + multi-teammate sync + presence** (hub/ Deno).
4. Expand the UI journey to cover session rail/resume/add-goal/multi-session and
   vault browser (E2E breadth).

**Resume protocol:** `git log --oneline -40` shows every step (prefixed
`feat: [sessions|vault|voice] …`, `merge: [voice|devex] …`). Re-run `make verify`
(+ `make hub-e2e`, `make accept`), then continue at the first unbuilt item above.
Plans live in `specs/NNN-*/plan.md`. 005/006 plans must be authored before build.

## How to resume after a compaction

1. `git log --oneline -15` — each commit = a completed step.
2. Read § Current position + the active plan's step checkboxes.
3. `make verify` — green = safe to continue.
4. Resume at the first unchecked step. TDD → green → commit → update this file.

## Key invariants / gotchas (don't relearn these the hard way)

- `wagner://run` events already carry `run_id` **inside** the `Run` payload →
  keying sessions is reducer-only; no new event channels. `memory/002`.
- A `Run` **is** a session — extend it, don't add a `Session` entity. `memory/002`.
- `AgentPool` is a **trait** (`edge/host/src/orchestrator/run_loop.rs:24`) → the
  loop is testable with a fake agent (`edge/host/tests/goal_loop.rs`). This is how
  acceptance tests run deterministically with no live CLI. `memory/003`.
- No native-shell E2E on macOS (`tauri-driver` unsupported). Acceptance = real
  engine (fake agent) + real UI via the `?mock` transport seam. `memory/003`.
- `tauri-plugin-dialog` + `@tauri-apps/plugin-dialog` already installed (picker is
  free). React Flow (`@xyflow/react`) is NOT installed — add for Phase 4.
- Run-state writes validate against `edge/host/schemas/run-state.schema.json` —
  adding `Run` fields means updating that schema too.
- Latent bug (fix in Phase 0–2): `edge/ui/transport/ipc.ts:51` invokes
  `wagner_${kind}`; real commands are `steer`/`abort`/`answer_transmission`.
- `make verify` = clippy cargo shell typecheck ts edge-build hub. UI journey
  (`edge-ui`) not yet in it — Phase A adds the `accept` gate.

## Blockers

(none yet)
