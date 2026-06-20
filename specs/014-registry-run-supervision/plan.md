# Implementation Plan: Registry Run Supervision

**Feature Branch:** `014-registry-run-supervision`
**Date:** 2026-06-19
**Spec:** [spec.md](./spec.md)
**Constitution:** `docs/spec/constitution.md` v0.1.0

This plan describes HOW the feature will be built. The WHAT lives in spec.md. Tasks (DO-THIS) live in tasks.md.

## Summary

Fold the shell's per-run map (`RunManager`/`RunControl`) onto `bus::AgentRegistry`, making the registry the single supervisor of every live participant — runs included. The goal loop stays an **imperative coroutine** (`run_goal`); the registry *supervises* it (owns its task + permission-gate handle + steering inbox + a cancel signal) and routes `Abort`/`Steer` to it through the validated command intake (P3). This is the supervised-coroutine model the deep-research recommended (hosting a run-to-completion job inside a reactive `handle()` starves control-message processing). The fact-publishing logic that today lives twice (shell `spawn_run_loop` and `GoalLoopAgent`) consolidates into `GoalLoopAgent`, which publishes identity-stamped facts via `AgentContext`. Separately (US2/P2), `Run`/`WagnerEvent`/`ModelProgress` gain `JsonSchema` derives so the typed stream gets real schemas. Strangler-fig: `make verify` + `make accept` stay green at every step.

---

## Technical Context

| Field | Value |
|-------|-------|
| Language / Version | Rust 2021 edition (host); TypeScript strict (shared contracts) |
| Primary Dependencies | `tokio` (broadcast/mpsc/**watch or Notify for cancel** — core, no new crate), `async-trait`, `ulid`, `chrono`, `serde`, `schemars` (JsonSchema), `thiserror`, `futures`; `tauri` (shell only) |
| Storage | Append-only per-run event log → `state.json` projections (D-STORE-2, Article VIII). **No new datastore.** |
| Testing | `cargo test` (host, scripted runner per D-TEST-1 — never spawns a real CLI); `vitest` (shared contracts, D-TEST-2); `make accept` acceptance journey; schema tests (D-TEST-3) |
| Target Platform | macOS desktop (Tauri edge app) |
| Project Type | Multi-package monorepo: `edge/host` (Rust crate) · `edge/shell` (Tauri) · `shared/` (TS) |
| Performance Goals | No new wall-clock target — abort is a **logical guarantee** (FR-013), not a latency SLO. Channels already bounded (P5); concurrency capped by `scheduler::default_concurrency` |
| Constraints | Swap-in-place migration; `make verify` + `make accept` green at every step; no new project/service/engine (Article V); edge runs stay offline-capable (Article VI) |
| Scale / Scope | Single operator, multiple concurrent runs (existing). Touches `edge/host/src/bus` + `orchestrator`, `edge/shell/src/commands.rs` + `bus_gateway.rs` + `gate.rs`, `shared/` contract regen |

> No `NEEDS CLARIFICATION` in Technical Context — the cancel-primitive choice is resolved in Phase 0 below.

---

## Constitution Check

Gates map to `docs/spec/constitution.md` Articles I–X.

- [x] **Gate I — Test-First:** tasks.md lists every test task before the implementation it covers (registry router, cancel-interrupt, duplicate-start guard, schema round-trip). Carried tests reused: `aborted_marks_run_terminal`, registry spawn/stop, goal-loop-to-Met.
- [x] **Gate II — Evidence-Driven:** spec vague-adjective scan clean ("promptly" → FR-013 logical guarantee). This plan names no unquantified adjectives.
- [x] **Gate III — CRITICAL-Resolved:** none open.
- [x] **Gate IV — Independent MVP:** US1 (P1) Independent Test is concrete (abort→terminal via the carried test + green gates).
- [x] **Gate V — Simplicity:** reuses existing `Bus`/`AgentRegistry`/`dispatch`; adds **no** project/service/engine and **no new crate** (cancel uses tokio core `watch`/`Notify`). The registry's per-entry value gains a run-bundle variant — a struct field, not a new subsystem.
- [x] **Gate VI — Edge-Executes-Hub-Remembers:** no hub on the critical path; the migration is edge-local. Offline-completion path unchanged.
- [x] **Gate VII — One-Directional Dependency:** work is in `edge/*` + `shared/` (which edge consumes). No library→platform dependency introduced.
- [x] **Gate VIII — Event-Sourced:** the aborted/halted terminal state is published as a `Snapshot` event folded by the UI's pure reducer (FR-006) — never delivered out-of-band. Gate VIII's verify criterion (replay-equals-snapshot) is satisfied by a **new** reducer-replay test for the aborted terminal (tasks T032) — not self-certified. *(The host persists run snapshots directly; the event-sourced spine the constitution names is the UI reducer that folds the emitted events. Making the host internally event-sourced is a pre-existing, out-of-014 concern.)*
- [x] **Gate IX — Privacy-Boundary:** no sync-path change; no code/transcript leaves the edge.
- [x] **Gate X — Schema-Validated:** US2 derives `JsonSchema` for `Run`/`WagnerEvent`/`ModelProgress` and regenerates `edge/host/schemas/` + TS; the command intake already schema-validates at the JSON boundary (P3).

**All gates pass — Complexity Tracking is empty.**

---

## Project Structure

```
edge/host/src/
├── bus/
│   ├── registry.rs        # ADD: supervised run-bundle (task+gate+console+cancel);
│   │                      #      spawn_run() with duplicate-name guard (FR-015);
│   │                      #      cancel(run_id) / steer(run_id,text);
│   │                      #      serve_commands(rx) — the P4 command router (FR-001)
│   ├── runtime.rs         # take_commands() now consumed by the registry router
│   ├── command.rs         # RunCommand::{Start,Abort,Steer} (exists); add Resume if needed
│   └── dispatch.rs        # intake unchanged (authz seam, Article IX)
├── orchestrator/
│   ├── run_loop.rs        # ADD: select! cancel-signal vs driver await → interrupt the
│   │                      #      in-flight turn (FR-013), drop driver (kill_on_drop)
│   ├── goal_loop_agent.rs # CONSOLIDATE: re-add steering (console) + cancel/halt; publish
│   │                      #      facts via AgentContext; this replaces shell spawn_run_loop
│   (Run lives in state/run.rs; aborted()/halted() helpers in shell commands.rs — unchanged behaviour)
├── events/model.rs        # WagnerEvent → derive JsonSchema (US2; currently Serialize/Deserialize only)
├── voice/models.rs        # ModelProgress → derive JsonSchema (US2; currently Serialize/Deserialize only)
└── state/run.rs           # Run → derive JsonSchema (US2)  [verified: opaque `data: unknown` in shared/contracts]
edge/shell/src/
├── commands.rs            # start_run/resume_run assemble deps → registry.spawn_run;
│                          #   abort/steer → dispatch only (effect inverts to the router);
│                          #   RunManager + RunControl + spawn_run_loop REMOVED
├── bus_gateway.rs         # spawn(): drop the take_commands() drain (router owns it now)
└── gate.rs                # gate server handle handed to the registry bundle, not RunControl
shared/                    # regen TS contracts (gen:contracts) for the tightened schemas
```

**Structure Decision:** No new directories — the work lands in the existing bus/orchestrator/shell modules and regenerates `shared/` contracts. This satisfies Gate V (smallest layout) and Gate VII (one-directional). The registry becoming the single supervisor (deleting `RunManager`) is the structural payoff: one authority for "who's live," per FR-002.

---

## Phase 0 — Research

The strategic question was answered by the deep-research workflow (run `wf_0e6e7ad3-0b1`, 27 sources, 15 verified claims). Key decisions it settled:

- **Decision:** Keep the goal loop an imperative supervised coroutine; do NOT make it a reactive `Agent` that hosts the run inside `handle()`.
  **Rationale:** A run-to-completion job in a reactive handler starves the participant's mailbox — it cannot process abort/steer for the run's duration (Akka Diagnostics, Petabridge, ractor docs; high-confidence). The canonical fix everywhere is "supervise + relay control signals," which is the model here.
  **Alternatives rejected:** Full reactive inversion (mailbox-starvation anti-pattern); command-broadcast routing (CQRS: commands must not be broadcast as events — can't be rejected/addressed; Cosmic Python ch.10).

- **Decision:** Deliver abort as **cooperative cancellation that interrupts the in-flight turn** — a `tokio::sync` cancel signal (`watch`/`Notify`, core — no new crate) the run loop `select!`s against its driver await; on cancel, drop the driver (`kill_on_drop` terminates the CLI child) and emit the terminal `Aborted` from the loop.
  **Rationale:** Satisfies FR-013 (don't wait for the in-flight turn) AND the spec assumption (terminal state originates from the loop, not a shell reconstruction). Avoids `JoinHandle::abort()` as normal control flow (Oxide RFD 0400 — caveated as one team's view, but the select!-on-cancel pattern is the broadly-valid alternative). Temporal's cooperative-cancellation model corroborates.
  **Alternatives rejected:** Hard `task.abort()` (loop can't emit its own terminal; the research's flagged anti-pattern); pure iteration-boundary flag (waits for the in-flight turn — fails FR-013); `tokio-util::CancellationToken` (adds a crate where tokio-core `Notify`/`watch` suffices — Gate V).

> `research.md` is optional. The full report lives in the workflow output; offer to persist a pointer if the engineer wants it in-tree.

---

## Phase 1 — Design & Contracts

### 1.1 Data Model

No new **persisted** entities; `Run` is unchanged on disk. The new structure is **in-memory** and replaces the shell's `RunControl`:

- **Supervised run-bundle (registry-owned):** `{ task: JoinHandle, gate_server: JoinHandle, console: Arc<Mutex<Vec<ConsoleInput>>>, cancel: <tokio cancel signal> }`, keyed by participant name `goal-loop:{run_id}`. Lifecycle: created by `spawn_run`; `cancel(run_id)` signals + the loop emits terminal `Aborted`; the bundle's gate_server is aborted with the run (no leak). Replaces `RunManager.runs`.

### 1.2 Interface Contracts

- **Command intake (existing, P3):** `RunCommand::{Start, Abort, Steer}` dispatched through `Bus::dispatch` (validate → authorize → stamp). Schema `command.v1`. `Abort`/`Steer` are addressed by `run_id`; rejectable (authz/backpressure). FR-001 chokepoint.
- **`AgentRegistry` additions (host, cargo-tested):**
  - `spawn_run(name, run_future, sidecars, cancel)` — supervises a run-to-completion future + sidecars; rejects if `is_running(name)` (FR-015, EC-005).
  - `cancel(run_id)` — signals the run's cancel; abort-wins over a pending steer (FR-014).
  - `steer(run_id, text)` — pushes into the run's console inbox (FR-005).
  - `serve_commands(rx)` — drains `take_commands()`; routes `Steer`→`steer` and acks `Start`. `Abort` is delivered directly for guaranteed stoppability (see effect-delivery decision); if an `Abort` also arrives via the queue, `cancel` is idempotent (a no-op on an already-terminal run, per EC-002). This is the deferred P4 router.
- **Tightened schemas (US2):** `Run`, `WagnerEvent`, `ModelProgress` derive `JsonSchema`; `gen:contracts` regenerates `edge/host/schemas/*.json` + `shared/` TS. Additive — variants stop being schema-opaque (FR-010/011).

**Start/abort/steer effect delivery (decision — revised per challenge C2/M1):** `start_run` dispatches `Start` (authz/audit) and, on `Accepted`, assembles Tauri deps and calls `registry.spawn_run` directly — start is not bus-spawned (deps are Tauri-coupled; spec Assumption). `steer` dispatches `Steer`, routed by `serve_commands` (a steer lost under intake backpressure is benign — the loop re-reads the console next iteration). **`abort` is guaranteed-stoppable:** it dispatches `Abort` for authorization, then — whether the bounded intake accepts or backpressures — delivers its effect by calling `registry.cancel(run_id)` **directly** (an authorized abort never waits in the queue). FR-001 holds (every action is authorized at the intake); FR-003/EC-007 hold (a saturated intake can never leave a run un-abortable). Only a *denied* abort (authz failure) does not cancel. `cancel` is idempotent, so a duplicate arriving via the queue is a no-op.

### 1.3 Cross-Cutting Concerns

#### Observability

- **Log fields:** carry the existing `eprintln!` lines (`agent '{name}' lagged: {n}`, `command accepted: {cmd}`); add `run cancelled: {run_id}` and `command routed: {type} → {run_id}` in `serve_commands`.
- **Metrics:** none — no metrics system yet (`D-OBS-2` placeholder, not in catalog → not added).
- **Trace spans:** none (in-process).

#### Security

- **Trust boundary:** the command intake. JSON commands validate against `command.v1` (Article X); typed commands are well-formed by construction. Then `CommandAuthorizer` authorizes (Article IX seam; v1 `AllowAll`, single operator).
- **Authentication / Authorisation:** unchanged — `AllowAll` v1; the seam tightens without touching call sites.
- **Secrets:** none introduced. Edge sets no metered API key (Article VI / D-SEC-1) — unaffected.
- **Gate-server ownership:** the loopback permission server handle moves into the run-bundle so an aborted run terminates its gate server (closes the current leak surface where it lived in `RunControl`).

#### Failure Modes

- **Run always stoppable (revised per challenge C2/M1):** `abort` dispatches for authorization, then calls `registry.cancel(run_id)` **directly** — its effect never waits in the bounded intake, so a saturated queue cannot drop or delay an abort (FR-003/EC-007). A backpressure-abort test (T033) asserts the run still stops when `dispatch` returns `Backpressure`. Steer (benign if lost) and start stay queue/shell-routed.
- **In-flight interrupt:** `cancel` wakes the loop's `select!` mid-turn; the driver is dropped (`kill_on_drop` kills the CLI child); the loop emits terminal `Aborted` (FR-013/FR-006).
- **Abort beats steer:** the loop checks cancel before draining the console (FR-014).
- **Duplicate start:** `spawn_run` guards on `is_running` and rejects, leaving the live run untouched (FR-015).
- **One handler error never stops the bus:** carried from the registry's existing drain loop.
- **Atomic terminal write:** load → `aborted()`/`halted()` → save (temp→validate→rename, D-RES-1) → publish; a partial aborted-state is never visible.

---

## Complexity Tracking

> All gates passed — no entries.

| Violated Gate | Why Needed | Simpler Alternative Rejected Because |
|---------------|------------|--------------------------------------|
| *(none)* | | |

---

## Optional Artifacts

- [ ] `data-model.md` — not needed (no new persisted schema; the run-bundle is in §1.1).
- [ ] `contracts/*.yaml` — not needed (commands/schemas are generated from Rust types via `gen:contracts`).
- [ ] `research.md` — available on request (full report in workflow `wf_0e6e7ad3-0b1`).
- [ ] `quickstart.md` — candidate: a `make gui-smoke` walkthrough for the native abort path (can't run headlessly; see Risks).

---

## Risks & Deferred (carried from handoff + this design)

- **Surgery on the live goal loop** — `run_loop.rs` gains a `select!` cancel branch. Do swap-in-place; run `make gui-smoke` on a workstation (the native abort path can't run headlessly).
- **Abort-through-intake backpressure** — RESOLVED (challenge C2): abort's effect bypasses the bounded queue via a direct `registry.cancel`, gated on intake authorization. Test T033 covers it. (No longer a deferred risk.)
- **`AgentRegistry::spawn()` same-name-replace footgun** (challenge H3) — `spawn()` aborts a same-named live participant; a future caller using a run-keyed name in `spawn()` would silently kill a live run. `spawn_run` is the only sanctioned path for run-keyed names; T034 adds a guard + test.
- **Graceful participant shutdown** (beyond run abort) — deferred until a participant holds an external resource needing a clean `shutdown`.
- **Real git-worktree isolation** for overlapping writers — still serialized (P5 deferral).
