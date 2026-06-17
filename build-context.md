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

1. **Commit each green step to `main`.** One atomic commit per plan step, ONLY
   when `make verify` is green. **Never commit red.** No pushes (no remote).
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

- [ ] **Phase A — Foundation**: context layer (this file + memory/) + acceptance
      harness (`make accept`, UI journey in gate) + 003 red acceptance tests.
- [ ] **Phase 0–2 — Sessions** (`specs/003-durable-concurrent-sessions`): entry
      redesign (folder picker + goal; drop guardrails grid/test field) + durable
      resume + concurrent sessions (RunManager map, run_id-keyed reducer, resume/
      list/get/add_goal IPC, Run-as-session fields). **← current critical path.**
- [ ] **Phase 3 — Vault v1** (`specs/004-vault-v1`): wikilinks + typed
      relationships + backlinks over `.wagner/memory`, deterministic linker.
      Extends `edge/host/src/memory.rs`. Parallel-capable.
- [ ] **Phase 4 — Graph + Vault browser** (`specs/005-graph-view`): React Flow
      over the vault (`@xyflow/react` to add). Depends on Phase 3.
- [ ] **Phase 5 — Distributed sync v1** (`specs/006-sync`): loro per-note + iroh
      gossip/docs + the file↔CRDT projector (highest risk). Depends on Phase 3.
- [ ] **Phase 6 — Hub vault store + multi-teammate sync + presence**.
- [ ] **Voice** (`specs/007-voice`): faster-whisper STT + Kokoro TTS, `voice/`
      seam. Fully parallel (handoff §7).

## Current position

**Phase A in progress.** Created the durable-context layer (this file +
`memory/`). Next: commit it, fan out background planning agents (Vault, Voice),
build the acceptance harness + 003 red tests, then execute `specs/003-*/plan.md`.

**Immediate next actions:**
1. Commit the context layer to `main`.
2. (bg, sonnet) author `specs/004-vault-v1/plan.md` + `specs/007-voice/plan.md`.
3. (main) Build `make accept` gate + promote UI journey; write 003 acceptance
   tests (red); then execute `specs/003-*/plan.md` step by step, commit per step.

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
