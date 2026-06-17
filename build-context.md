# Wagner — autonomous build context (live resume anchor)

> **Single source of truth for an unattended, autonomous build.**
> Fresh agent (post-compaction or new session): read this top to bottom, then
> `memory/000-index.md`, then resume at **§ Current position**. Keep this file
> current — update it at the end of every step.
>
> (Not to be confused with `CONTEXT.md` — that's the domain glossary.)

Last updated: 2026-06-17 (autonomous build kickoff)

## Mission

Build the **entire Wagner product** autonomously, overnight, committing each
green step to `main`. No questions to the operator unless a hard blocker forces
it. The operator is asleep. Decisions come from the locked plan + sensible
defaults and are recorded in `memory/`.

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
- [~] **Phase 3 — Vault v1** (`specs/004-vault-v1`): IN PROGRESS. Step 1 DONE
      (`03fbde4` — note frontmatter summary/tier/lifecycle/provenance scalars on
      MemoryRecord, projected when set). Next: Step 2 (deterministic `[[wikilink]]`
      parser in new `edge/host/src/vault/linker.rs`). Steps 3–8 per the plan.
      NOTE: SurrealDB 2.x rejects enums/Option/nested → persisted fields are plain
      scalars; typed relationships go in a separate table (Step 3), not on
      MemoryRecord.
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

**Now building: Plan 004 — Vault v1.** Step 1 committed (`03fbde4`). Next is
Step 2 (wikilink parser → new `edge/host/src/vault/linker.rs`; create the
`vault` module + `pub mod vault;` in `edge/host/src/lib.rs`; parse `[[Name]]` and
`[[actual|alias]]`, skip code fences/inline code — pulldown-cmark or a careful
std scanner). Then steps 3–8, then Plan 005 (Graph, needs React Flow `@xyflow/
react`), Plan 006 (Sync, hard), Plan 007 (Voice, parallel).

**Resume protocol:** `git log --oneline -25` shows every committed step (one per
commit, prefixed `feat: [sessions] 003 step N` / `feat: [vault] 004 step N`).
Re-run `make verify`, then continue at the first unbuilt step named above.

Commit map — 003: step1 `af9d9d5`, step2 `3ae100b`, step3 `6b7a106`, step4
`5e3880c`, step5 `61356b9`, step6 `5066d98`, steps7-8 `f14d6f5`, step9 `76ee180`,
step10 `0164ce7`. 004: step1 `03fbde4`.

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
