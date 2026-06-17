---
title: A Run IS a session (no separate Session entity)
summary: Extend the existing Run aggregate into the session; event payloads already carry run_id, so multi-session keying is reducer-only — no new event channels.
tags: [decision, architecture, sessions, engine]
tier: core
---

# A Run IS a session

**Decided:** 2026-06-17 during Plan 003 design (recon-grounded).

`Run` (`edge/host/src/state/run.rs`) is already durable (persisted every
iteration to `{app_data}/runs/{id}/state.json`), resumable (`state::load` +
`RunStatus::Paused` exist), and bound to a goal. So **a Run is the session.** Add
the missing fields (`project_dir`, `name`, `updated_at`, `goals: Vec`) rather than
inventing a `Session` entity on top. Kills an abstraction (ponytail/YAGNI).

**Two recon findings that shrink the work:**
1. `wagner://run` events emit the whole `Run`, which **already contains `run_id`**.
   So keying many sessions in the UI is a **reducer-only** reshape
   (`run: RunSnapshot|null` → `runs: Record<RunId, RunSnapshot>`, key on
   `payload.run_id`). No new event channels, no Rust emit-site refactor. The
   handoff hedged "channels or discriminator" — the discriminator already exists.
2. Adding `Run` fields requires updating `edge/host/schemas/run-state.schema.json`
   (writes validate against it) — make `goals` default `[]` so old runs still load.

**Goal model (the handoff's flagged-vague piece):** `goals` is a flat append log
folded into planning context; `add_goal` reuses the resume path to reactivate a
finished/paused session. Ceiling: per-goal independent lifecycle if ever needed.

**How to apply:** see `specs/003-durable-concurrent-sessions/plan.md` steps 1, 5,
7. Related: [[003-acceptance-harness-strategy]].
