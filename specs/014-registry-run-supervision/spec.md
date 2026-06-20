# Feature Specification: Registry Run Supervision

**Feature Branch:** `014-registry-run-supervision`
**Created:** 2026-06-19
**Status:** Draft
**Input:** Engineer's original description: continue the 011 runtime foundation — the deferred P4 integration. Fold the shell's `RunManager`/`spawn_run_loop` onto `bus::AgentRegistry` so the goal loop runs as a supervised participant in the running app, with start/abort/steer routed through the validated command intake; tighten the opaque event variants (Run/WagnerEvent/ModelProgress) so the typed stream gets real schemas. Direction set by deep-research (wf_0e6e7ad3-0b1): keep the goal loop an **imperative supervised coroutine**, not a reactive bus participant.

> **Spec authoring rules** (from the project constitution):
> - Focus on **WHAT** users need and **WHY**. No HOW — that lives in plan.md.
> - Every requirement must be testable and unambiguous (Article II).
> - User stories are prioritised P1/P2/P3 and each must be independently testable (Article IV).
> - **No silent defaults.** Defaults applied only from `docs/spec/defaults.md`, cited by ID. Everything else the engineer didn't specify is `[NEEDS CLARIFICATION]`.

---

## User Scenarios & Testing *(mandatory)*

User stories are ordered by priority. P1 is the MVP — if you implement only User Story 1 you must still ship something that delivers the value this feature promises.

### User Story 1 — Run lifecycle owned by one supervisor (Priority: P1)

The operator starts, steers, and aborts autonomous runs exactly as they do today, but every run-control action now flows through the single validated command intake and is delivered to the run by one central supervisor that owns every live participant. There is no longer a second, ad-hoc map of running tasks living in the shell. A new agent or connector becomes live through that same supervisor path — the goal loop is just one participant among many, not a hard-wired special case.

**Why this priority:** This is the deferred 011 P4 integration and the load-bearing inversion: until the run lifecycle is owned by the registry, "new agent = new integration = same move" is not true in the running app, abort/steer remain bespoke shell code, and the goal loop stays the hard-wired centre. It delivers the platform's pluggability promise.

**Independent Test:** Start a run, then abort it through the command path; the run reaches a terminal `Aborted` state on its event stream and every UI surface leaves "running" — verified by (a) the carried `aborted_marks_run_terminal` test passing through the new registry-routed path AND (b) a reducer-replay test asserting the aborted-terminal projection equals a fold of the run's events (FR-006/Article VIII; the carried test alone is not the Article VIII evidence), plus `make verify` + `make accept` green.

**Acceptance Scenarios:**

1. **Given** a live run, **When** the operator aborts it, **Then** the run transitions to terminal `Aborted` on its event stream, the UI leaves "running", and the run's own loop emits the terminal state (not an out-of-band reconstruction).
2. **Given** two concurrently live runs A and B, **When** the operator aborts run A, **Then** run A terminates and run B keeps running.
3. **Given** a live run, **When** the operator submits a steering instruction, **Then** the instruction is delivered to that run and reflected in its next iteration.
4. **Given** a run-control action (start/abort/steer), **When** it is issued, **Then** it is authorized at the single validated command intake before it takes effect; no action takes effect without intake authorization (effect delivery may be local to the owning authority).
5. **Given** a new agent or connector, **When** it joins the running system, **Then** it registers and is supervised through the same registry path the goal loop uses, with no bespoke per-participant lifecycle code.
6. **Given** an unauthorized or schema-invalid run-control command, **When** it is dispatched, **Then** it is rejected at the intake and no live run is affected.
7. **Given** a live run whose permission gate has been blocked past its timeout, **When** the timeout fires, **Then** the run is promoted to a terminal halted state on its event stream (FR-007).
8. **Given** a run that is already live, **When** a start/resume for the same `run_id` is issued, **Then** the live run keeps running untouched and the duplicate is rejected/no-op (FR-015).
9. **Given** the carried run-control test suite and the `?mock` acceptance journey, **When** the migration is complete, **Then** all previously-passing assertions pass unchanged (no assertion edited) — evidencing behavioural equivalence (FR-012).
10. **Given** an aborted run's event log, **When** it is replayed through the UI's pure reducer from empty, **Then** the projection equals the live `Aborted` snapshot (FR-006, Article VIII).
11. **Given** a saturated command intake, **When** an authorized abort is issued, **Then** the run is still stopped (the abort's effect reaches the registry directly) and the abort is not dropped (FR-003, EC-007).

---

### User Story 2 — Typed event payloads carry real schemas (Priority: P2)

The run, activity, and model-progress payloads that flow to the UI validate against declared JSON Schemas instead of being wrapped schema-opaque. This closes the Article X gap left open during the 011 migration and lets the typed event reducer fold these payloads directly (the prerequisite the surface port, out of scope here, depends on).

**Why this priority:** Independent and additive — it ships alone, touches no run-control path, and unblocks the deferred surface port. Carrying opaque variants indefinitely violates Article X (schema-validated payloads) at the UI boundary.

**Independent Test:** Regenerate the contracts and assert that a representative `Run`, `WagnerEvent`, and `ModelProgress` payload each validates against its declared schema and round-trips through the generated TypeScript contract — verified by a schema-validation test (D-TEST-3) and `make verify` green.

**Acceptance Scenarios:**

1. **Given** a `Run` / `WagnerEvent` / `ModelProgress` payload, **When** it is emitted to a UI surface, **Then** it validates against a declared JSON Schema (draft 2020-12) before emission.
2. **Given** the generated TypeScript contracts, **When** the typed event reducer folds a run/activity/progress event, **Then** the payload is a real typed shape, not an opaque blob.

---

### Edge Cases

- **EC-001 (concurrency):** An abort for run A arrives while a steering instruction for run A is still in flight. Expected: the abort wins — the run reaches terminal `Aborted` and the pending steer is discarded (FR-014). Steering a run being terminated is meaningless.
- **EC-002 (boundary):** Abort issued for a `run_id` that is not live (already completed/terminal). Expected: no-op success, no error (carried `abort_targets` filters to live ids).
- **EC-003 (failure / responsiveness):** The run's current CLI subprocess is mid-execution when an abort arrives. Expected: the run is terminated **without awaiting completion of the in-flight CLI turn** — that turn's output is discarded, no further turn is started, and the run reaches terminal `Aborted` (FR-013). Wall-clock process teardown is OS-immediate (process-kill-on-drop) and is not separately targeted. *(The mechanism itself is plan.md.)*
- **EC-004 (zero-state):** Abort issued when no runs are live. Expected: no-op success.
- **EC-005 (concurrency):** A start/resume targets a `run_id` that is already live. Expected: the supervisor rejects (or no-ops) it — the live run is untouched, never killed-and-restarted (FR-015). This overrides the registry's bare same-name-replaces default for runs.
- **EC-006 (malicious / invalid input):** A schema-invalid or unauthorized run-control command is dispatched. Expected: rejected at the intake; no live run is started, steered, or aborted (FR-009).
- **EC-007 (failure — control-plane saturation):** The command intake is saturated (`dispatch` returns `Backpressure`/`NoConsumer`) when an abort is issued. Expected: the abort is still authorized and its effect reaches the registry directly so the run is stopped — the abort is never silently dropped (FR-003). A denied abort (authorization failure) is rejected, not dropped (FR-009).

---

## Functional Requirements *(mandatory)*

- **FR-001:** Every run-control action (start, abort, steer) MUST be authorized at the single validated command intake (validate → authorize → stamp) before it takes effect; no run-control action may take effect without passing intake authorization. *(Effect delivery is local to the authority that owns it — start spawns shell-side after authz, abort/steer reach the registry after authz — but authorization always happens at the intake; see plan §1.2. This is an authz/audit chokepoint, not a single delivery queue.)*
- **FR-002:** The system MUST track all live participants (runs, connectors, scheduler) in one registry; there MUST NOT be a second, separate registry/map of live runs in the shell.
- **FR-003:** Aborting a run MUST terminate that run and transition it to a terminal `Aborted` state observable on its event stream; every UI surface MUST leave the "running" state. An abort with no target specified MUST abort every live run (the single-session UI default); an abort targeting a specific `run_id` MUST abort only that run. Abort MUST remain effective even when the command intake is saturated — an authorized abort's effect MUST reach the registry directly rather than waiting in a bounded queue, so a full intake can never leave a run un-abortable (see EC-007, plan §Failure Modes).
- **FR-004:** Aborting one run MUST NOT affect any other concurrently live run.
- **FR-005:** A steering instruction submitted to a live run MUST be delivered to that run and applied on its next iteration.
- **FR-006:** The terminal state of an aborted or halted run MUST reach the UI as a `Snapshot` event on the run's stream — folded by the UI's pure reducer like every other run event — never delivered by an out-of-band channel (Article VIII). A test MUST assert the aborted-terminal projection is produced by replaying the run's events through the pure reducer (Gate VIII verify criterion).
- **FR-007:** A blocked-permission timeout MUST promote the affected run to a terminal halted state (carried T042/FR-016 behaviour).
- **FR-008:** A new agent or connector MUST register and be supervised through the same registry path the goal loop uses; adding a participant MUST NOT require modifying the run-control or lifecycle code paths.
- **FR-009:** The command intake MUST reject an unauthorized or schema-invalid run-control command and MUST leave every live run unaffected by the rejected command.
- **FR-010:** `Run`, `WagnerEvent`, and `ModelProgress` payloads MUST validate against declared JSON Schemas (draft 2020-12, `additionalProperties: false` per the catalog default) before emission to a UI surface (Article X).
- **FR-011:** The generated TypeScript contracts MUST expose the tightened `Run`/`WagnerEvent`/`ModelProgress` schemas so the typed event reducer can fold these payloads as real typed shapes.
- **FR-012:** The migration MUST preserve the existing operator-observable run behaviour: concurrent sessions, steering, blocked-timeout halt, and abort-leaves-running. Preservation MUST be evidenced by the carried run-control test suite and the `?mock` acceptance journey passing unchanged — no edits to their assertions (US1-AS9). *(Migration scope — see Assumptions for the swap-in-place rationale and the one intentional change: an aborted run's terminal state now originates from the run's own loop on the event stream.)*
- **FR-013:** Abort MUST NOT wait for the run's in-flight CLI turn to complete; the in-flight turn's output MUST be discarded and no further turn started before the run reaches terminal `Aborted`.
- **FR-014:** When an abort and a steering instruction for the same run are both pending, the abort MUST take priority — the run MUST reach terminal `Aborted` and the pending steer MUST be discarded.
- **FR-015:** A start/resume targeting a `run_id` that is already live MUST NOT terminate or restart the live run; the supervisor MUST reject or no-op the request, leaving the running session untouched.

---

## Success Criteria *(mandatory)*

- **SC-001:** An operator can abort a running session and observe it reach terminal `Aborted` with no UI surface still showing "running", in 100% of abort attempts.
- **SC-002:** With N concurrent sessions live, aborting one leaves the other N−1 running (no cross-run termination).
- **SC-003:** 100% of run-control actions (start, abort, steer) pass through the single intake; zero run-control bypass paths remain in the shell.
- **SC-004:** A new participant is added to the running system with zero changes to the run-control or lifecycle code paths (registration only).
- **SC-005:** 100% of `Run`/`WagnerEvent`/`ModelProgress` payloads emitted to the UI validate against a declared schema.
- **SC-006:** The existing acceptance journey and full verification suite pass unchanged after the migration — zero regressions in previously-passing behaviour. *(Verification method — the `make verify` / `make accept` gates — is recorded in plan.md, not named here.)*

---

## Key Entities

- **Run:** One autonomous session — goal, guardrails, event stream, outcome, learnings. Lifecycle: `Running` → terminal `{Met, Halted, Aborted}`. State derived from its append-only event log (Article VIII, D-STORE-2).
- **Participant:** Any live actor on the bus — a run's goal loop, a connector, the scheduler, the UI. Has a lifecycle (register → run → stop) and a stable logical name.
- **Registry (supervisor):** The single authority that knows which participants are live and owns their lifecycle (start/stop). Replaces the shell's per-run map.
- **Command:** A validated, imperative, authorizable intent (`start`/`abort`/`steer`) carried on the intake — addressable, rejectable, distinct from an event.
- **Event / Fact:** A past-tense fact (activity, snapshot, panel, terminal state) broadcast on the run's stream and folded by the pure reducer.

---

## Assumptions

- The goal loop remains an **imperative coroutine** that is *supervised* (its lifecycle owned by the registry, control signals routed to it); it is NOT re-implemented as a reactive `handle()`-loop participant. (Engineer decision from deep-research wf_0e6e7ad3-0b1: hosting a run-to-completion job inside a reactive message handler starves the participant's ability to process abort/steer for the run's duration.)
- The validated command intake (011 P3 `dispatch`/`take_commands`), the `AgentRegistry` (011 P4), and `GoalLoopAgent` already exist in `edge/host`; this work integrates them into the running app rather than building them.
- A run's execution dependencies (app-data paths, the US2 permission-gate server, the CLI agent pool, recall) are assembled in the Tauri shell and remain shell-assembled; `start` is therefore not a pure hub/bus command.
- Run-control behaviour observable to the operator (concurrent sessions, steering, blocked-timeout halt) must be preserved exactly (FR-012).

### Defaults Applied

- `D-OBS-1` — applied to FR-006/FR-010 and Key Entities (every run emits a normalized append-only event stream; observation/steering read from it).
- `D-STORE-2` — applied to FR-006 and the Run entity (run state is a projection of the append-only event log; Article VIII).
- `D-RES-1` — applied to FR-003/FR-006 (terminal-state writes are atomic: write-temp → validate → rename; a partial aborted-state write is never visible).
- `D-TEST-1` — applied to US1 Independent Test and SC-006 (Rust host tested with `cargo test`; the loop runs over a scripted runner — tests never spawn a real CLI).
- `D-TEST-3` — applied to US2 Independent Test / FR-010 (schema-validation test against each committed schema + a representative payload).
- `D-SEC-3` / Article X — applied to FR-009/FR-010 (external inputs and emitted payloads validate against a declared schema).
- `D-PROJ-2` — applied to US1 (a participant/run is modeled over the shared run/event spine, not a new subsystem).
- `D-PROJ-3` — applied throughout (platform vocabulary: agent/participant, not floor-era terms).
- `D-PROJ-4` — applied to scope (edge stack stays Rust host + TypeScript frontend / Tauri).

### Defaults Overridden

- *(empty)*

---

## Out of Scope

- **Surface port (011 P7 / "C"):** pointing `edge/ui` at the typed stream and porting the six locked mock surfaces, plus UI `dispatch(Command)` wiring. Blocked on the re-mocked surfaces landing in the repo (operator's explicit "UI wiring last"). US2 prepares the schemas it depends on; the port itself is a follow-on spec.
- **Full reactive-`Agent` inversion of the goal loop** (goal loop hosts its run inside `handle()`): refuted by deep-research; explicitly not pursued.
- **Making `start` a pure bus/hub command:** the run's deps are Tauri-coupled; `start` stays shell-assembled.
- **Real git-worktree isolation for overlapping writers** (011 P5 deferral; currently serialized).
- **Graceful shutdown for participants that hold external resources** (a cancellation signal beyond run abort): deferred until a participant actually holds such a resource.

---

## Dependencies

- **011 P3 command intake (`dispatch` / `take_commands`):** provides the single validated path commands route through. If absent, FR-001 cannot hold. (Exists on `main`.)
- **011 P4 `AgentRegistry`:** the supervisor the run lifecycle folds onto. (Exists on `main`.)
- **`GoalLoopAgent`:** the existing wrapper that publishes loop facts as bus events. (Exists on `main`.)
- **US2 permission-gate server (`start_gate_server`):** the loopback permission server bound per run; its lifecycle handle must move under registry ownership so abort terminates it. If it leaks, an aborted run's gate server lingers.
- **Re-mocked UI surfaces:** external dependency that blocks the out-of-scope surface port only; does not block US1 or US2.

---

## Cross-Plugin Surfaces

| Layer | Obligations | Owner |
|-------|--------------|-------|
| `edge/host/` | Command router over `take_commands()` (the deferred P4 routing core); registry owns the run bundle (task + gate + steering inbox + cancel signal); tightened `JsonSchema` on `Run`/`WagnerEvent`/`ModelProgress` | engine |
| `edge/shell/` | `start_run`/`resume_run`/`abort`/`steer` route through `dispatch` → registry; `RunManager` removed; gate-server handle moves under the registry entry | shell |
| `shared/` | Regenerated TypeScript contracts expose the tightened schemas for the typed reducer | shared |

---

## Constitution Addenda *(optional)*

- *(none — all governing rules are in the project constitution.)*

---

## Clarifications

Filled in by `/spec clarify`.

### Session 2026-06-19

- Q: Which story is the P1 MVP? → A: US1 (registry run lifecycle) = P1; US2 (typed schemas) = P2, independent additive increment.
- Q: How is abort responsiveness quantified (replacing "promptly")? → A: Logical guarantee (FR-013) — abort discards the in-flight CLI turn and starts no further turn before reaching Aborted; no wall-clock target (OS kill is immediate, untestable under scripted-runner D-TEST-1).
- Q: Abort-vs-steer ordering for the same run (EC-001)? → A: Abort always wins (FR-014); the pending steer is discarded.
- Q: Start/resume targeting an already-live run_id (EC-005)? → A: Protect the live run (FR-015) — supervisor rejects/no-ops; the running session is untouched.

### Session 2026-06-19 — Challenge dispositions (spec-challenger, all ACCEPTED)

- C1/C3 (Article VIII / test-coverage): FR-006 reworded — terminal reaches the UI as a `Snapshot` event folded by the pure reducer; added US1-AS10 + a reducer-replay test task (the carried `aborted_marks_run_terminal` is no longer the sole Article VIII evidence).
- C2/M1 (abort backpressure): FR-003 + EC-007 + US1-AS11 — an authorized abort's effect reaches the registry directly (not the bounded queue), so a saturated intake can never leave a run un-abortable. Added a backpressure-abort test task.
- H1 (FR-001 "no bypass"): reworded to an authz-at-intake chokepoint (effect delivery is local to the owning authority); US1-AS4 aligned.
- H2 (FR-012 coverage): reworded to require the carried suite + `?mock` journey pass unchanged; added US1-AS9.
- H3 (registry `spawn()` footgun): added a task to make `spawn_run` the only path for run-keyed names + a guard test.
