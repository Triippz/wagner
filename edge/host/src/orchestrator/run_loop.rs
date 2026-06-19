//! The autonomous goal loop (T034, FR-008/011/013).
//!
//! Each iteration:
//!   1. guardrail check (iterations, cost) → halt if tripped
//!   2. Oracle plan pass (Architects/Claude) → parse + validate the plan
//!   3. if the plan hypothesizes goal-met → run the suite + judge pass → decide
//!   4. else dispatch each planned subtask to its assigned engine, collect results
//!   5. fold cost, bump iteration, persist run-state
//!
//! Orchestrates over `EngineRunner` so it is fully testable with scripted engines.

use super::engine::{EngineOutcome, EngineRunner, Role};
use super::guardrails::{self, Verdict};
use super::judge::{self, GoalVerdict, JudgeInputs, SuiteResult};
use super::oracle;
use crate::events::Faction;
use crate::state::{Run, RunStatus, Subtask, SubtaskState};
use std::path::Path;

/// The deployed roster as the loop sees it: look up the runner, floor identity,
/// and faction for each hired agent by id, plus the lead (Claude) agent that runs
/// the plan + judge passes. The real impl wraps `Roster` + per-agent CLI runners;
/// tests provide a scripted pool.
pub trait AgentPool: Send + Sync {
    /// Id of the lead (Claude) agent that runs the plan + judge passes.
    fn lead_id(&self) -> String;
    /// The runner for an agent id (`None` if not in the roster).
    fn runner(&self, agent_id: &str) -> Option<&dyn EngineRunner>;
    /// All hired agent ids (for plan validation + the plan prompt).
    fn ids(&self) -> Vec<String>;
    /// Floor faction (colour) for an agent id.
    fn faction(&self, agent_id: &str) -> Faction;
    /// Display name for an agent id (the floor label).
    fn name(&self, agent_id: &str) -> String;
    /// Human-readable roster listing injected into the plan prompt.
    fn brief(&self) -> String;
}

/// Dependencies the loop needs from the outside world, injected for testability.
///
/// NOTE (architecture ceiling): `emit`, `progress`, and `emit_panel` are three
/// IPC sinks that differ only in payload. If a fourth observable is needed,
/// collapse them into a single typed `EventSink` rather than adding a field —
/// this struct should not grow past its current shape.
pub struct LoopDeps<'a> {
    /// The hired-agent roster: who can be assigned work, and who leads.
    pub pool: &'a dyn AgentPool,
    /// Runs the full project suite (cargo + vitest + playwright). Async so the
    /// real impl can offload the blocking shell-out to a blocking thread
    /// (`spawn_blocking`) without starving the runtime; the fake returns instantly.
    pub run_suite: &'a (dyn Fn() -> futures::future::BoxFuture<'static, SuiteResult> + Send + Sync),
    /// Where to persist run-state each iteration.
    pub runs_root: &'a Path,
    /// Sink for projected `WagnerEvent`s (drives the floor). No-op in tests.
    pub emit: &'a (dyn Fn(crate::events::WagnerEvent) + Send + Sync),
    /// Drains any steering instructions the engineer submitted since the last
    /// iteration (US3, FR-009). Returns them in submission order; empty if none.
    pub steer: &'a (dyn Fn() -> Vec<crate::state::ConsoleInput> + Send + Sync),
    /// Reports an out-of-band halt the loop must honor — set by the US2 gate when
    /// a permission transmission times out (blocked too long, FR-016). Checked at
    /// the top of each iteration; `Some(reason)` halts the whole run (T042).
    pub external_halt: &'a (dyn Fn() -> Option<crate::state::HaltReason> + Send + Sync),
    /// Live run-snapshot sink — called after each phase/iteration change so the
    /// mission bar reflects "what's happening now" without waiting for the run to
    /// end (the final snapshot is still emitted by the caller). No-op in tests.
    pub progress: &'a (dyn Fn(&Run) + Send + Sync),
    /// Sink for an agent-authored UI-spec panel (Phase 5): `(operative_id, raw
    /// spec JSON)`. Called when an agent emits a ```ui-spec block; the frontend
    /// validates/sanitizes before rendering. No-op in tests.
    pub emit_panel: &'a (dyn Fn(&str, serde_json::Value) + Send + Sync),
}

/// Project an engine outcome's signals into events for one hired agent and emit
/// them. The operative on the floor is the agent (id + display name), coloured by
/// its faction.
fn emit_outcome(
    deps: &LoopDeps<'_>,
    run_id: &str,
    operative_id: &str,
    operative_name: &str,
    faction: Faction,
    signals: &[crate::events::CliSignal],
) {
    use crate::events::{signal_to_event, EventContext};
    for sig in signals {
        if let Some(ev) = signal_to_event(
            sig,
            EventContext {
                run_id,
                operative_id,
                operative_name,
                faction,
                event_id: ulid::Ulid::new().to_string(),
                ts: chrono::Utc::now().to_rfc3339(),
            },
        ) {
            (deps.emit)(ev);
        }
    }
}

/// If an agent emitted a ```ui-spec panel in its output, forward the raw JSON to
/// the panel sink (the frontend validates + sanitizes before rendering).
fn emit_panel_if_any(deps: &LoopDeps<'_>, operative_id: &str, final_text: &str) {
    if let Some(spec) = super::panel::extract_panel(final_text) {
        (deps.emit_panel)(operative_id, spec);
    }
}

/// Persist run-state, LOGGING (never silently dropping) a write failure — a lost
/// state write must leave a diagnosis trail rather than vanishing (M7).
fn save_run(runs_root: &Path, run: &Run) {
    if let Err(e) = crate::state::save(runs_root, run) {
        eprintln!("[wagner] run-state save failed for run {}: {e}", run.run_id);
    }
}

/// Run the goal loop to completion (met / halted). Returns the final `Run`.
pub async fn run_goal(mut run: Run, deps: LoopDeps<'_>) -> Run {
    use crate::state::RunPhase;
    run.status = RunStatus::Running;
    run.phase = RunPhase::Planning;
    save_run(deps.runs_root, &run);
    (deps.progress)(&run);

    // Index into `run.subtasks` where the most recent dispatch batch begins.
    // Goal-met convergence is judged over the LATEST batch only, so a subtask
    // that failed in an earlier iteration cannot permanently block `Met` once a
    // later iteration supersedes it (M5).
    let mut last_batch_start = 0usize;

    loop {
        // 0. Drain live steering into the run so this iteration's plan pass sees it.
        for input in (deps.steer)() {
            run.console_inputs.push(input);
        }

        // 0b. External halt — the US2 gate timed out a transmission (FR-016).
        if let Some(reason) = (deps.external_halt)() {
            run.status = RunStatus::HaltedGuardrail;
            run.phase = RunPhase::Halted;
            run.halt_reason = Some(reason);
            save_run(deps.runs_root, &run);
            (deps.progress)(&run);
            return run;
        }

        // 1. Guardrails.
        if let Verdict::Halt(reason) = guardrails::check(&run.guardrails, run.guardrails.cost.used)
        {
            run.status = RunStatus::HaltedGuardrail;
            run.phase = RunPhase::Halted;
            run.halt_reason = Some(reason);
            save_run(deps.runs_root, &run);
            (deps.progress)(&run);
            return run;
        }

        // 2. Oracle plan pass — run by the lead (Claude) agent.
        run.phase = RunPhase::Planning;
        (deps.progress)(&run);
        let lead_id = deps.pool.lead_id();
        let lead_name = deps.pool.name(&lead_id);
        let lead_faction = deps.pool.faction(&lead_id);
        let Some(lead) = deps.pool.runner(&lead_id) else {
            // A misconfigured run (no lead-capable agent) must end as a terminal
            // halt, not panic the loop task (M6).
            eprintln!("[wagner] run {} has no lead agent in the roster — halting", run.run_id);
            run.status = RunStatus::HaltedGuardrail;
            run.phase = RunPhase::Halted;
            run.halt_reason = Some(crate::state::HaltReason::Misconfigured);
            save_run(deps.runs_root, &run);
            (deps.progress)(&run);
            return run;
        };
        let roster_ids = deps.pool.ids();

        let plan_out = lead
            .run(Role::Plan, &plan_prompt(&run, &deps.pool.brief()))
            .await;
        run.guardrails.cost.used += plan_out.cost;
        emit_outcome(
            &deps,
            &run.run_id,
            &lead_id,
            &lead_name,
            lead_faction,
            &plan_out.signals,
        );
        emit_panel_if_any(&deps, &lead_id, &plan_out.final_text);

        let plan = match oracle::parse_plan(&plan_out.final_text, &roster_ids) {
            Ok(p) => p,
            Err(e) => {
                // Log the parse failure so a stuck oracle leaves a diagnosis trail.
                eprintln!("[wagner] run {} oracle plan parse failed, re-prompting: {e}", run.run_id);
                // One re-prompt, then give up this iteration (escalation lands with US2).
                let retry = lead.run(Role::Plan, &replan_prompt(&run)).await;
                run.guardrails.cost.used += retry.cost;
                emit_outcome(
                    &deps,
                    &run.run_id,
                    &lead_id,
                    &lead_name,
                    lead_faction,
                    &retry.signals,
                );
                emit_panel_if_any(&deps, &lead_id, &retry.final_text);
                match oracle::parse_plan(&retry.final_text, &roster_ids) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("[wagner] run {} oracle re-plan parse failed, skipping iteration: {e}", run.run_id);
                        run.guardrails.iterations_used += 1;
                        run.iteration += 1;
                        save_run(deps.runs_root, &run);
                        continue;
                    }
                }
            }
        };

        // 3. Goal-met hypothesis → verify with suite + judge (lead agent).
        if plan.goal_met_hypothesis {
            run.phase = RunPhase::Judging;
            (deps.progress)(&run);
            let suite = (deps.run_suite)().await;
            let judge_out = lead.run(Role::Judge, &judge_prompt(&run)).await;
            run.guardrails.cost.used += judge_out.cost;
            emit_outcome(
                &deps,
                &run.run_id,
                &lead_id,
                &lead_name,
                lead_faction,
                &judge_out.signals,
            );
            emit_panel_if_any(&deps, &lead_id, &judge_out.final_text);
            let verdict = judge::decide(JudgeInputs {
                all_subtasks_done: run.subtasks[last_batch_start..]
                    .iter()
                    .all(|s| s.state == SubtaskState::Done),
                suite,
                claude_confirms: confirms(&judge_out.final_text),
            });
            if verdict == GoalVerdict::Met {
                run.status = RunStatus::Met;
                run.phase = RunPhase::Met;
                save_run(deps.runs_root, &run);
                (deps.progress)(&run);
                return run;
            }
        }

        // 4. Dispatch each planned subtask to its assigned agent.
        run.phase = RunPhase::Dispatching;
        (deps.progress)(&run);
        // This iteration's subtasks form a fresh batch; the next goal-met check
        // converges over them, not over superseded earlier-iteration subtasks (M5).
        last_batch_start = run.subtasks.len();
        // Dispatch the planned subtasks with bounded concurrency ("30 at once"):
        // disjoint + read-only subtasks run concurrently up to the scheduler cap;
        // subtasks whose declared write-paths overlap are serialized into separate
        // waves so they never clobber each other (R-ISOLATION).
        // ponytail: overlapping writers serialize; running them concurrently in
        // isolated git worktrees (Subtask.worktree) is the upgrade for throughput.
        let cap = super::scheduler::default_concurrency(
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2),
        );
        let write_paths: Vec<Vec<String>> =
            plan.subtasks.iter().map(|s| s.may_write_paths.clone()).collect();
        let mut outcomes: Vec<Option<EngineOutcome>> =
            (0..plan.subtasks.len()).map(|_| None).collect();
        for wave in super::scheduler::non_overlapping_waves(&write_paths) {
            let tasks: Vec<futures::future::BoxFuture<'_, (usize, EngineOutcome)>> = wave
                .into_iter()
                .filter_map(|i| {
                    // parse_plan guaranteed the agent is in the roster; skip
                    // defensively if not.
                    let planned = &plan.subtasks[i];
                    deps.pool.runner(&planned.agent).map(|runner| {
                        let desc = planned.description.as_str();
                        Box::pin(async move { (i, runner.run(Role::Execute, desc).await) })
                            as futures::future::BoxFuture<'_, (usize, EngineOutcome)>
                    })
                })
                .collect();
            for (i, outcome) in super::scheduler::run_bounded(cap, tasks).await {
                outcomes[i] = Some(outcome);
            }
        }
        // Fold results in plan order — deterministic event + subtask order,
        // unchanged from the sequential version (only execution is concurrent).
        for (i, planned) in plan.subtasks.iter().enumerate() {
            let Some(outcome) = outcomes[i].take() else {
                continue;
            };
            let id = format!("{}-{}-{}", run.run_id, run.iteration, i);
            let agent_id = &planned.agent;
            let faction = deps.pool.faction(agent_id);
            let agent_name = deps.pool.name(agent_id);
            run.guardrails.cost.used += outcome.cost;
            emit_outcome(
                &deps,
                &run.run_id,
                agent_id,
                &agent_name,
                faction,
                &outcome.signals,
            );
            emit_panel_if_any(&deps, agent_id, &outcome.final_text);
            run.subtasks.push(Subtask {
                id,
                agent_id: agent_id.clone(),
                assignment_rationale: Some(planned.assignment_rationale.clone()),
                prompt: planned.description.clone(),
                state: if outcome.success {
                    SubtaskState::Done
                } else {
                    SubtaskState::Failed
                },
                worktree: None,
                result_summary: Some(outcome.final_text),
                parent_event_ids: Vec::new(),
            });
        }

        // 5. Advance + persist.
        run.guardrails.iterations_used += 1;
        run.iteration += 1;
        save_run(deps.runs_root, &run);
        (deps.progress)(&run);
    }
}

/// Whether the judge's reply confirms goal-met. Requires a STRUCTURED verdict —
/// the JSON `{"met": true}` the judge prompt asks for, or an exact `GOAL_MET: YES`
/// token. Never a bare substring like "confirmed", which also matches
/// "not confirmed"/"unconfirmed" and would falsely unlock `Met`.
fn confirms(text: &str) -> bool {
    if text.contains("GOAL_MET: YES") {
        return true;
    }
    super::json_scan::balanced_objects(text)
        .into_iter()
        .filter_map(|o| serde_json::from_str::<serde_json::Value>(o).ok())
        .find_map(|v| v.get("met").and_then(|m| m.as_bool()))
        == Some(true)
}

/// The exact wire shape the Oracle must emit. Shown verbatim in the prompt —
/// real models otherwise improvise a richer plan shape that fails validation.
const PLAN_EXAMPLE: &str = r#"{"schema":"oracle-plan.v2","subtasks":[{"description":"what this subtask does","agent":"<an id from the ROSTER>","assignment_rationale":"why this agent","may_write_paths":["path/it/may/edit"],"depends_on":[]}],"goal_met_hypothesis":false}"#;

fn plan_prompt(run: &Run, roster_brief: &str) -> String {
    let steering = run
        .console_inputs
        .iter()
        .map(|c| format!("- {}", c.text))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are the Oracle for an autonomous build. Decompose the GOAL into subtasks and \
         assign each to one of the hired agents below by its `id`, matching the work to the \
         agent's role and engine.\n\n\
         Return ONLY a single JSON object — no prose, no markdown fences, no extra keys — of \
         EXACTLY this shape:\n{example}\n\n\
         Rules: use ONLY these keys (`description`, `agent`, `assignment_rationale`, \
         `may_write_paths`, `depends_on`); every `agent` MUST be one of the ROSTER ids; \
         `depends_on` holds indices of earlier subtasks or []. Set `goal_met_hypothesis` to \
         true with `subtasks` [] ONLY when the goal is already fully achieved.\n\n\
         ROSTER:\n{roster}\n\nGOAL:\n{goal}\n\nSTEERING:\n{steering}",
        example = PLAN_EXAMPLE,
        roster = roster_brief,
        goal = run.goal,
        steering = steering
    )
}

fn replan_prompt(run: &Run) -> String {
    format!(
        "Your previous reply was not valid oracle-plan.v2 JSON. Reply with ONLY this JSON \
         object shape, no prose, no markdown, no extra keys:\n{example}\n\n\
         Every `agent` must be a ROSTER id. GOAL:\n{goal}",
        example = PLAN_EXAMPLE,
        goal = run.goal
    )
}

fn judge_prompt(run: &Run) -> String {
    format!(
        "Has this goal been fully met by the work completed? Reply with a JSON object \
         {{\"met\": true|false}}. GOAL:\n{}",
        run.goal
    )
}
