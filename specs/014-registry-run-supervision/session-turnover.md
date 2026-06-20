# Session turnover — 014 registry run supervision

**Branch:** `014-registry-run-supervision` (from `origin/main` @ `88e3d7f`)
**Tip:** `8b711ee` · `git status` clean · `make verify` green at this commit.

## Done (committed, verified)

- **Full SDD spec** (`spec.md`/`plan.md`/`tasks.md`/`challenges.md`/`validation-report.md`) — validated READY; 7 challenge findings dispositioned.
- **US2 — typed schemas** (`150f778`): `JsonSchema` on `Run`/`WagnerEvent`/`ModelProgress`; opaque `RunEvent`/`VoiceEvent` variants tightened; contracts regenerated (`data: unknown` → typed). Green.
- **US1 host-side — registry + cooperative cancel** (`150f778` + `8b711ee`):
  - `AgentRegistry`: `spawn_run` / `cancel` / `abort_run` / `steer` / `spawn_guarded` / `spawn_run_and_drive` + run-bundle + run-keyed-name footgun guard.
  - `run_goal`: `LoopDeps.cancel` (`Option<watch::Receiver<bool>>`); `biased select!` races the drive loop vs cancel → drops the in-flight turn → terminal Aborted (FR-013).
  - `GoalLoopAgent::with_cancel`.
  - Tests green: `bus_registry` (11), `goal_loop_agent` (2), `bus_dispatch` (8), `run_cancel` (4: T005 cancel-interrupt, T006, T010, T033). Full host suite + `make verify` green.

## Remaining

### Shell rewire (T018–T021) — NOT started (reverted a partial attempt)
Migrate `edge/shell/src/commands.rs` off `RunManager` onto `AgentRegistry`. It is **larger than a uniform swap** and **behaviorally unverifiable without `make gui-smoke`** (native window), which is why it was deferred to a workstation session.

Sites (line numbers at `8b711ee`):
- `RunManager` (82) + `RunControl` (249) structs → delete.
- `spawn_run_loop` (131) → refactor into `build_run_future(SpawnLoop, gate_server, cancel_rx) -> impl Future` (set `LoopDeps.cancel: Some(cancel_rx)`; `gate_server.abort()` at the end) + a `register_run(&Arc<AgentRegistry>, run_id, SpawnLoop, gate_server)` helper that calls `registry.spawn_run(run_id, |rx| build_run_future(.., rx), steer_closure)`. The steer closure pushes into the shared `console` the loop drains (preserves US3 steering); `external_halt`/`blocked_halt` already wired (preserves T042).
- **start_run** (340), **resume_run** (454): `mgr: State<RunManager>` → `registry: State<Arc<AgentRegistry>>`; replace `spawn_run_loop(..)` + `mgr.runs.insert(.., RunControl{..})` with `register_run(registry.inner(), run_id.clone(), SpawnLoop{..}, gate.server_task)?`.
- **add_goal** (514): `is_live`/guard reads (526/528) → `registry.is_running` / `registry.steer`. (Appends a goal to a live run or resumes.)
- **start_workflow** (673): drives a *workflow* task (not the goal loop) — register its task via `registry.spawn_run` with a future that ignores the cancel (or `select!`s it to abort), `gate_server` torn down at the end. Different shape from the goal-loop sites.
- **steer** (857): `registry.steer(run_id, text)` (with the single/none/multi target logic preserved via `registry.running()`).
- **abort** (912): keep `dispatch(Run::Abort)` for authz, then call `registry.cancel(run_id)` **directly** for each target (challenge C2 — bypasses bounded intake so abort is always effective). Keep the `abort_targets`/`aborted` pure helpers (their unit tests stay green).
- **lib.rs** (55–86): drop `.manage(RunManager::default())`; create `Arc::new(AgentRegistry::new(bus.clone()))` (same bus Arc as `UiGateway`, before it's moved at line 86) and `app.manage(..)`.
- `serve_commands` is **not required** — the Tauri commands call the registry directly after dispatch-for-authz; the existing `bus_gateway::spawn` `take_commands` drain stays.

Verify after: `make verify` (compile + clippy `-D warnings` + shell cargo tests incl. `aborted_marks_run_terminal`/`abort_targets` + TS + Deno) and `make accept` (the `?mock` Playwright UI journey). Then **`make gui-smoke` on a workstation** (native window + a real run/abort) — the one residual that needs a display + Accessibility permission.

### Then
- Mark 011 P4 integration DONE in `specs/011-runtime-foundation/plan.md`.
- `/review` → `/finish` → PR.
