# Plan 003 — acceptance scenarios (definition of done)

> These Given/When/Then scenarios define "done" for the sessions wedge. Each is
> realized as a test during the step that implements it (TDD), so `make verify`
> (engine) + `make accept` (UI journey) green ⇒ these all hold. Column "Test"
> names where the assertion lands.

## Engine (Rust — `make verify` via cargo/shell, fake `AgentPool`)

| # | Given | When | Then | Test | Step |
|---|---|---|---|---|---|
| E1 | a fresh run is created with a project dir + first goal | it is persisted | the saved state carries `project_dir`, `name`, `updated_at`, and `goals=[first goal]` | `state/run.rs` + store test | 1 |
| E2 | an old run JSON without `goals` on disk | it is loaded | it loads (goals defaults to `[]`); a new run missing `project_dir` is rejected by the schema | store/schema test | 1 |
| E3 | two runs are started | both are live | both have independent control handles; neither replaces the other | `RunManager` map test | 2 |
| E4 | two live runs | one is aborted by id | only that run ends; the other keeps running | abort-by-id test | 2 |
| E5 | three runs persisted with different `updated_at` | `list_runs` is called | summaries return newest-first with the light shape; a corrupt run dir is skipped, not fatal | `store::list_summaries` test | 3 |
| E6 | a `Paused`/closed run on disk | `resume_run` is called | it rebuilds the pool from the persisted `project_dir` and re-enters the loop; status → `Running` | resume integration test (fake agent) | 4 |
| E7 | `start_run` and `resume_run` exist | both spawn the loop | they share one loop-spawn helper (no duplicated closure) | code structure / compile | 4 |
| E8 | a `Met`/`Paused` run | `add_goal` is called | the goal is appended and the run reactivates toward `Running`; the planner input includes the new goal | add_goal test | 5 |
| E9 | `start_run` invoked with guardrails omitted | the run starts | defaults are applied (no error) | command default test | 6 |

## UI (TypeScript — `make verify` via vitest; journey via `make accept`/edge-ui)

| # | Given | When | Then | Test | Step |
|---|---|---|---|---|---|
| U1 | two run snapshots with different `run_id` | both folded via `applyRun` | both coexist in `state.runs`; updating one doesn't clobber the other | `reducer.test.ts` | 7 |
| U2 | no session is focused | the first run arrives | it auto-focuses (`selectedRunId` set); `activeRun` returns it | `reducer.test.ts` | 7 |
| U3 | a control message (`steer`/`abort`/`answer`) is sent through the IPC transport | the bridge invokes | it calls the real command name (`steer`), not `wagner_steer` | `ipc.test.ts` | 6 |
| U4 | the focused session changes | `activeRun` re-resolves | TopBar/Inspector render the newly focused run | surface/reducer test | 7–8 |
| U5 | the new-session screen | it renders | it shows a native folder picker + a goal field; NO guardrails grid, NO test-command field | UI journey (ui-smoke) | 9 |
| U6 | several sessions exist (in-memory + from `list_runs`) | the rail renders | one row per session with a status dot (running/needs-you/idle/done); clicking a closed one reopens it; "New session" opens the composer | UI journey (ui-smoke) | 10 |

## Cross-cutting

- The baseline tests that asserted `state.run?.…` are migrated to the new shape
  (`activeRun(state)` / `state.runs[id]`) and still pass — `reducer.test.ts`,
  `run_view.test.ts`, `p2p.test.ts`. This is a deliberate contract change, not a
  regression.
- `make verify` green after every step; `make accept` green at the end.
