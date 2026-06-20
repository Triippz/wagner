# Tasks: Registry Run Supervision

**Feature Branch:** `014-registry-run-supervision`
**Inputs:** [spec.md](./spec.md) (US1 P1, US2 P2), [plan.md](./plan.md) (supervised-coroutine HOW), `docs/spec/constitution.md`

> **Constitution Article I (NON-NEGOTIABLE):** every behaviour-changing implementation task is preceded by a test task. Strangler-fig: `make verify` + `make accept` stay green at every checkpoint.

---

## Phase 1 — Setup

- [ ] T001 Create branch `014-registry-run-supervision` from `origin/main` (013 is merged; local `main` is stale → `git fetch && git checkout -b 014-registry-run-supervision origin/main`).
- [ ] T002 Confirm green baseline: `make verify` + `make accept` pass before any edit (records the regression bar for SC-006).

---

## Phase 2 — Foundational (Blocking Prerequisites)

> US1 and US2 are independent; the only shared prerequisite is the green baseline (T002). No further cross-cutting foundation is needed — keeping this phase thin satisfies Gate V.

- [ ] T003 Identify the carried tests to re-point (not rewrite): `aborted_marks_run_terminal`, the registry spawn/stop tests (`edge/host/src/bus/registry.rs`), the goal-loop-to-Met test (`edge/host/.../goal_loop_agent` tests), and the dispatch deny/invalid tests (`edge/host/src/bus/dispatch.rs`). Note their current locations for reuse in US1.

**Checkpoint:** Baseline green; carried tests catalogued. User-story phases can begin.

---

## Phase 3 — User Story 1 — Run lifecycle owned by one supervisor (Priority: P1) 🎯 MVP

**Goal:** The registry becomes the single supervisor of every live run; start/abort/steer route through the validated intake; `RunManager` is deleted; the goal loop stays an imperative coroutine. (FR-001–009, FR-012–015)
**Independent Test:** Start a run, abort it through the command path; the run reaches terminal `Aborted` on its event stream and every UI surface leaves "running" — the carried `aborted_marks_run_terminal` test passes through the new registry-routed path, plus `make verify` + `make accept` green.

### Tests for User Story 1 (write FIRST, ensure they FAIL)

- [ ] T004 [P] [US1] Router test — a dispatched `RunCommand::Abort{run_id}` routes to `cancel(run_id)` and `RunCommand::Steer{run_id,text}` routes to `steer(run_id,text)`, in `edge/host/src/bus/registry.rs` tests (FR-001, US1-AS4).
- [ ] T005 [P] [US1] Cancel-interrupt test — a run driven by the scripted runner, cancelled mid-turn, discards the in-flight turn output, starts no further turn, and emits a terminal `Aborted` snapshot from the loop, in `edge/host/tests/unit/run_cancel.rs` (FR-013, FR-006, FR-003, US1-AS1, EC-003).
- [ ] T006 [P] [US1] Abort-beats-steer test — with a cancel and a pending steer for the same run, the run reaches `Aborted` and the steer is discarded, in `edge/host/tests/unit/run_cancel.rs` (FR-014, EC-001).
- [ ] T007 [P] [US1] Duplicate-start guard test — `spawn_run` for an already-live name is rejected/no-op and the live run is untouched, in `edge/host/src/bus/registry.rs` tests (FR-015, EC-005, US1-AS8).
- [ ] T008 [P] [US1] Concurrent-abort isolation test — two live runs; aborting one leaves the other running, in `edge/host/src/bus/registry.rs` tests (FR-004, US1-AS2).
- [ ] T009 [P] [US1] Re-point the carried `aborted_marks_run_terminal` test so it exercises the registry-routed abort path (FR-003, US1-AS1).
- [ ] T010 [P] [US1] Blocked-timeout halt test — a gate blocked past its timeout promotes the run to a terminal halted state on its stream, in `edge/host/tests/unit/run_cancel.rs` (FR-007, US1-AS7).
- [ ] T011 [P] [US1] Reject test — an unauthorized or schema-invalid run-control command is rejected at the intake and no live run is affected (re-point dispatch deny/invalid tests), `edge/host/src/bus/dispatch.rs` (FR-009, US1-AS6).
- [ ] T012 [P] [US1] Pluggability test — a second participant registers and is supervised via the same registry path with no run-control code changed, in `edge/host/src/bus/registry.rs` tests (FR-008, US1-AS5).
- [ ] T013 [P] [US1] Goal-loop facts test — `GoalLoopAgent` drives a goal to `Met` publishing identity-stamped facts via `AgentContext`, with steering (console) and cancel/blocked-halt wired in, in the `goal_loop_agent` tests (FR-005, FR-006, US1-AS3).
- [ ] T032 [P] [US1] **(challenge C1/C3)** Reducer-replay test — replay an aborted run's event log from empty through the UI pure reducer and assert the projection equals the live `Aborted` snapshot, in `edge/ui/tests/unit/reducer.test.ts` (FR-006, Article VIII Gate-VIII verify, US1-AS10). This — not the carried `aborted_marks_run_terminal` — is the Article VIII evidence.
- [ ] T033 [P] [US1] **(challenge C2/M1)** Backpressure-abort test — with the command intake saturated so `Bus::dispatch(Abort)` returns `Backpressure`, assert the authorized abort still stops the run (effect reaches `registry.cancel` directly) and is not dropped, in `edge/host/tests/unit/run_cancel.rs` (FR-003, EC-007, US1-AS11).
- [ ] T034 [P] [US1] **(challenge H3)** Spawn-footgun guard test — assert `spawn_run` is the only sanctioned path for run-keyed names and a guard prevents bare `spawn()` from silently aborting a live run-keyed participant, in `edge/host/src/bus/registry.rs` tests (FR-015).

### Implementation for User Story 1

- [ ] T014 [US1] Add the cancel signal type + run-bundle `{task, gate_server, console, cancel}` to `AgentRegistry`; add `spawn_run(name, future, sidecars, cancel)` with the duplicate-name guard, `cancel(run_id)`, and `steer(run_id, text)`, in `edge/host/src/bus/registry.rs` (FR-002, FR-004, FR-005, FR-014, FR-015). Cancel uses tokio-core `watch`/`Notify` (no new crate).
- [ ] T015 [US1] Add `serve_commands(rx)` to `AgentRegistry` — drain `take_commands()`, route `Abort`→`cancel`, `Steer`→`steer`, ack `Start`; one handler error never stops the loop, in `edge/host/src/bus/registry.rs` (FR-001, FR-009).
- [ ] T016 [US1] Add the cancel `select!` to the driver await in `edge/host/src/orchestrator/run_loop.rs` — interrupt the in-flight turn, drop the driver (`kill_on_drop` kills the CLI child), and return the terminal so the loop emits `Aborted` (FR-013, FR-006, FR-003).
- [ ] T017 [US1] Consolidate fact-publishing into `GoalLoopAgent` (`edge/host/src/orchestrator/goal_loop_agent.rs`) — re-add steering (drain `console`) and cancel/blocked-halt; publish all loop facts via `AgentContext`. This becomes the single loop-driver that the shell `spawn_run_loop` duplicated (FR-005, FR-006).
- [ ] T018 [US1] Move the permission-gate-server handle into the registry run-bundle so aborting a run terminates its gate server (`edge/host/src/bus/registry.rs` + `edge/shell/src/gate.rs`) — closes the leak (FR-003, plan §Security).
- [ ] T019 [US1] Wire the shell: `start_run`/`resume_run` dispatch `Start` then call `registry.spawn_run`; `steer` dispatches `Steer` (router-routed); **`abort` dispatches `Abort` for authz then calls `registry.cancel(run_id)` directly** so it is stoppable even under intake backpressure (challenge C2); delete `RunManager`, `RunControl`, and `spawn_run_loop`, in `edge/shell/src/commands.rs` (FR-001, FR-002, FR-003, FR-012, FR-015).
- [ ] T035 [US1] **(challenge H3)** Add the run-keyed-name guard in `AgentRegistry` so `spawn()` cannot silently replace a live run-keyed participant (run lifecycle goes only through `spawn_run`), in `edge/host/src/bus/registry.rs` (FR-015; makes T034 pass).
- [ ] T020 [US1] Move the `take_commands()` drain out of `bus_gateway::spawn` and wire `registry.serve_commands(bus.take_commands())` at app setup, in `edge/shell/src/bus_gateway.rs` + `edge/shell/src/lib.rs` (FR-001).
- [ ] T021 [US1] Add observability: `run cancelled: {run_id}` and `command routed: {type} → {run_id}` log lines in `serve_commands`/`cancel` (plan §1.3 Observability).

**Checkpoint:** US1 Independent Test passes end-to-end — abort via the command path reaches terminal `Aborted`, UI leaves "running", carried `aborted_marks_run_terminal` green through the new path. `make verify` + `make accept` green; run `make gui-smoke` on a workstation (native abort path can't run headless — plan §Risks).

---

## Phase 4 — User Story 2 — Typed event payloads carry real schemas (Priority: P2)

**Goal:** `Run`/`WagnerEvent`/`ModelProgress` validate against declared JSON Schemas and the generated TS contracts expose them, so the typed reducer folds real shapes (not `data: unknown`). (FR-010, FR-011)
**Independent Test:** Regenerate contracts; a representative `Run`, `WagnerEvent`, and `ModelProgress` each validate against their declared schema and round-trip the generated TypeScript contract; `make verify` green.

### Tests for User Story 2 (write FIRST, ensure they FAIL)

- [ ] T022 [P] [US2] Schema round-trip test — `Run`, `WagnerEvent`, `ModelProgress` each validate against their declared JSON Schema and serde round-trips, in `edge/host/tests/unit/schema_roundtrip.rs` (FR-010, US2-AS1, D-TEST-3).
- [ ] T023 [P] [US2] Reducer-folds-typed test — the typed reducer folds a run/activity/progress event as a real typed shape (not `unknown`), in `edge/ui/tests/unit/reducer.test.ts` (FR-011, US2-AS2).

### Implementation for User Story 2

- [ ] T024 [P] [US2] Derive `schemars::JsonSchema` on `Run` in `edge/host/src/state/run.rs` (FR-010).
- [ ] T025 [P] [US2] Derive `JsonSchema` on `WagnerEvent` in `edge/host/src/events/model.rs` (FR-010).
- [ ] T026 [P] [US2] Derive `JsonSchema` on `ModelProgress` in `edge/host/src/voice/models.rs` (FR-010).
- [ ] T027 [US2] Tighten the opaque leaf variants (`RunEvent::{Snapshot,Activity,…}`, `VoiceEvent::DownloadProgress`) so they carry the typed schema instead of opaque pass-through, where the contract is generated (FR-010, FR-011).
- [ ] T028 [US2] Regenerate contracts (`gen:contracts`) → `edge/host/schemas/*.json` + `shared/contracts/*.d.ts`; confirm `data: unknown` becomes the typed shape (FR-011).

**Checkpoint:** US2 Independent Test passes — schemas validate + round-trip TS; `make verify` green. US1 still green (independent).

---

## Final Phase — Polish & Cross-Cutting Concerns

- [ ] T029 Mark the 011 P4 integration complete in `specs/011-runtime-foundation/plan.md` (the deferred shell-adoption is now done).
- [ ] T030 Code cleanup / refactor per the `/tdd` Refactor phase — remove any dead shell glue left after `RunManager` deletion; confirm no `spawn_run_loop` references remain (`rg spawn_run_loop`).
- [ ] T031 Run the native abort-path walkthrough (`make gui-smoke`) on a workstation and record the result (plan §Risks — quickstart candidate).

---

## Dependencies & Execution Order

### Phase Dependencies
- **Setup (1)** → first.
- **Foundational (2)** → depends on Setup; thin (baseline + test catalogue).
- **US1 (3)** and **US2 (4)** → both depend only on Foundational; **independent of each other** (US1 = bus/orchestrator/shell; US2 = schema derives + contracts). Can run in parallel / either order.
- **Polish (Final)** → depends on the stories shipped.

### Within US1 (strangler-fig, keep green)
- Tests T004–T013 + T032–T034 precede implementation T014–T021 + T035.
- T014 (registry surface) precedes T015 (router), T035 (guard), and T019/T020 (shell wiring on it).
- T016 (loop cancel) + T017 (GoalLoopAgent consolidation) precede T019 (shell deletes `spawn_run_loop`).
- T021 (observability) before the Checkpoint.
- T032–T034 are `[P]` (distinct files: `edge/ui` reducer, `run_cancel.rs`, `registry.rs`).

### Parallel Opportunities
- US1 tests T004–T013 are `[P]` (distinct test files / modules).
- US2 derive tasks T024–T026 are `[P]` (three distinct source files).
- US1 and US2 phases can be worked concurrently by separate agents after Foundational.

---

## Implementation Strategy

**MVP first:** Setup → Foundational → US1 → stop and run the US1 Independent Test + `make gui-smoke`. That ships the registry adoption (the headline value) alone.

**Then US2:** the additive schema tightening — independently shippable, unblocks the (out-of-scope) surface port.

---

## Coverage Check (FR → task)

| FR | Tasks | FR | Tasks |
|----|-------|----|-------|
| FR-001 | T004,T015,T019,T020 | FR-009 | T011,T015 |
| FR-002 | T014,T019 | FR-010 | T022,T024,T025,T026,T027 |
| FR-003 | T005,T009,T016,T018,T019,**T033** | FR-011 | T023,T027,T028 |
| FR-004 | T008,T014 | FR-012 | T002 baseline + carried suite (Checkpoint) + US1-AS9 |
| FR-005 | T013,T014,T017 | FR-013 | T005,T016 |
| FR-006 | T005,T013,T016,T017,**T032** | FR-014 | T006,T014 |
| FR-007 | T010 | FR-015 | T007,T014,T019,**T034,T035** |
| FR-008 | T012 | EC-007 | **T033** |

Every FR and every acceptance scenario (US1-AS1…AS11, US2-AS1…AS2) has ≥1 task; every implementation task is preceded by a test task; every task names a concrete path. Challenge additions: T032 (Article VIII replay), T033 (backpressure abort), T034/T035 (spawn footgun guard).
