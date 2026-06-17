---
title: Roadmap DAG and parallelization lanes
summary: Phase dependency order and which lanes can run as parallel cost-efficient subagents vs the serialized main-loop critical path on main.
tags: [decision, process, parallelization, roadmap]
tier: core
---

# Roadmap DAG & parallelization

**Decided:** 2026-06-17.

## Dependency order

```
Phase A (foundation: context + acceptance harness)
   └─ Phase 0–2 Sessions (003) ── critical path, serialized on main
Phase 3 Vault v1 (004) ── engine-side, independent of session UI ── parallel-capable
   ├─ Phase 4 Graph (005) ── needs vault
   └─ Phase 5 Sync (006) ── needs vault ── then Phase 6 Hub
Voice (007) ── fully independent ── parallel-capable
```

## Parallelization rules

- **Main loop owns `main`** and the critical path (TDD + gated commits).
  Commits MUST be serialized — never two writers on one branch unattended.
- **Parallel subagents (cheap models, sonnet)** are allowed for:
  - Authoring NEW spec/plan/doc files (no code-file collision) — e.g. planning
    Vault and Voice while sessions are built.
  - Read-only recon and code review at gates.
  - Isolated implementation in **git worktrees** for genuinely independent phases
    (Voice; Vault engine) — each verified in its lane, landed on main one at a
    time by the main loop.
- **Model routing:** sonnet for recon/planning/mechanical; opus for the hard
  gated build and risky components (the sync projector).

## Failure modes to avoid (from parallel-execution-optimizer)

- Concurrency that creates conflicting edits on `main`.
- Calling "fast" done before acceptance is green.
- Forgetting to poll background agents/sessions.

Related: [[001-autonomous-build-mandate]], [[003-acceptance-harness-strategy]].
