//! T025 — integration test: the full goal loop over scripted engines reaches
//! `Met` when the plan hypothesizes completion and the suite + judge agree, and
//! `HaltedGuardrail` when iterations run out without a met verdict.
//!
//! No real CLI is spawned — `EngineRunner` is scripted, exercising the loop's
//! orchestration logic deterministically.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use wagner_edge_host::state::{ConsoleInput, HaltReason, Run, RunStatus};

/// A two-agent test pool: lead `cipher` (Claude/architect) + `vex` (Codex/forger),
/// mirroring the original relay so the scripted runners drive the loop unchanged.
struct TwoAgentPool<'a> {
    architect: &'a dyn EngineRunner,
    forger: &'a dyn EngineRunner,
}

impl AgentPool for TwoAgentPool<'_> {
    fn lead_id(&self) -> String {
        "cipher".into()
    }
    fn runner(&self, agent_id: &str) -> Option<&dyn EngineRunner> {
        match agent_id {
            "cipher" => Some(self.architect),
            "vex" => Some(self.forger),
            _ => None,
        }
    }
    fn ids(&self) -> Vec<String> {
        vec!["cipher".into(), "vex".into()]
    }
    fn faction(&self, agent_id: &str) -> Faction {
        if agent_id == "vex" {
            Faction::Forgers
        } else {
            Faction::Architects
        }
    }
    fn name(&self, agent_id: &str) -> String {
        agent_id.into()
    }
    fn brief(&self) -> String {
        "cipher (Cipher) — architect [claude]\nvex (Vex) — forger [codex]".into()
    }
}

/// No-op steer source for tests that don't exercise live steering.
fn no_steer() -> Vec<ConsoleInput> {
    Vec::new()
}

/// No external halt — the default for tests not exercising the US2 blocked-timeout
/// promotion (T042).
fn no_halt() -> Option<HaltReason> {
    None
}

fn no_progress(_r: &Run) {}

fn no_panel(_id: &str, _spec: serde_json::Value) {}

/// An architect that plans one real subtask on iteration 0, then claims goal-met
/// on iteration 1 and confirms as judge.
struct ScriptedArchitect {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for ScriptedArchitect {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let met = n >= 1;
                let json = if met {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[
                        {"description":"impl the thing","agent":"vex","assignment_rationale":"scoped impl","may_write_paths":["src/x.rs"],"depends_on":[]}
                    ],"goal_met_hypothesis":false}"#
                };
                EngineOutcome {
                    signals: vec![],
                    success: true,
                    cost: 0.01,
                    final_text: json.to_string(),
                }
            }
            Role::Judge => EngineOutcome {
                signals: vec![],
                success: true,
                cost: 0.01,
                final_text: r#"{"met": true}"#.to_string(),
            },
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

/// A forger that always succeeds.
struct ScriptedForger;

#[async_trait]
impl EngineRunner for ScriptedForger {
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome {
            signals: vec![],
            success: true,
            cost: 5.0,
            final_text: "implemented".to_string(),
        }
    }
}

/// An architect that never claims goal-met (forces the guardrail path).
struct NeverDoneArchitect;

#[async_trait]
impl EngineRunner for NeverDoneArchitect {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        let json = r#"{"schema":"oracle-plan.v2","subtasks":[
            {"description":"keep going","agent":"vex","assignment_rationale":"x","may_write_paths":[],"depends_on":[]}
        ],"goal_met_hypothesis":false}"#;
        let text = if role == Role::Plan { json } else { "{}" };
        EngineOutcome {
            signals: vec![],
            success: true,
            cost: 0.0,
            final_text: text.to_string(),
        }
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wagner-loop-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[tokio::test]
async fn loop_reaches_met_when_plan_suite_and_judge_agree() {
    let root = temp_root("met");
    let run = Run::new(
        "01J000000000000000000000ME".into(),
        "build the thing".into(),
        vec![],
        "2026-06-13T00:00:00Z".into(),
    );
    let architect = ScriptedArchitect {
        plan_calls: AtomicUsize::new(0),
    };
    let forger = ScriptedForger;
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };

    let pool = TwoAgentPool {
        architect: &architect,
        forger: &forger,
    };
    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    assert_eq!(final_run.status, RunStatus::Met);
    // One subtask was dispatched on the first iteration before goal-met on the second.
    assert_eq!(final_run.subtasks.len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn loop_halts_on_max_iterations_when_never_met() {
    let root = temp_root("halt");
    let mut run = Run::new(
        "01J000000000000000000000HA".into(),
        "impossible goal".into(),
        vec![],
        "2026-06-13T00:00:00Z".into(),
    );
    run.guardrails.max_iterations = Some(3);
    let architect = NeverDoneArchitect;
    let forger = ScriptedForger;
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: false } }) };

    let pool = TwoAgentPool {
        architect: &architect,
        forger: &forger,
    };
    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    assert_eq!(final_run.status, RunStatus::HaltedGuardrail);
    assert_eq!(final_run.guardrails.iterations_used, 3);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn loop_halts_on_cost_budget() {
    let root = temp_root("cost");
    let mut run = Run::new(
        "01J000000000000000000000CO".into(),
        "expensive goal".into(),
        vec![],
        "2026-06-13T00:00:00Z".into(),
    );
    run.guardrails.max_iterations = Some(1000);
    run.guardrails.cost.budget = Some(8.0); // forger costs 5/subtask → trips fast
    let architect = NeverDoneArchitect;
    let forger = ScriptedForger;
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: false } }) };

    let pool = TwoAgentPool {
        architect: &architect,
        forger: &forger,
    };
    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    assert_eq!(final_run.status, RunStatus::HaltedGuardrail);
    assert_eq!(
        final_run.halt_reason,
        Some(wagner_edge_host::state::HaltReason::Cost)
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An architect that records every prompt it is asked to plan against, and
/// claims goal-met on its second plan pass.
struct RecordingArchitect {
    plan_calls: AtomicUsize,
    plan_prompts: Mutex<Vec<String>>,
}

#[async_trait]
impl EngineRunner for RecordingArchitect {
    async fn run(&self, role: Role, prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                self.plan_prompts.lock().unwrap().push(prompt.to_string());
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let json = if n >= 1 {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":false}"#
                };
                EngineOutcome {
                    signals: vec![],
                    success: true,
                    cost: 0.0,
                    final_text: json.to_string(),
                }
            }
            Role::Judge => EngineOutcome {
                signals: vec![],
                success: true,
                cost: 0.0,
                final_text: r#"{"met": true}"#.to_string(),
            },
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

/// T054 — a steering instruction injected into the live queue is drained into
/// the run and reaches the next Oracle plan prompt (US3, FR-009).
#[tokio::test]
async fn loop_drains_live_steer_into_next_plan_prompt() {
    let root = temp_root("steer");
    let run = Run::new(
        "01J000000000000000000000ST".into(),
        "build a feature".into(),
        vec![],
        "2026-06-13T00:00:00Z".into(),
    );
    let architect = RecordingArchitect {
        plan_calls: AtomicUsize::new(0),
        plan_prompts: Mutex::new(Vec::new()),
    };
    let forger = ScriptedForger;
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };

    // The steer source yields one instruction on the first drain, then nothing.
    let pending = Mutex::new(vec![ConsoleInput {
        ts: "2026-06-13T00:00:01Z".into(),
        text: "also update the README".into(),
    }]);
    let steer = || std::mem::take(&mut *pending.lock().unwrap());

    let pool = TwoAgentPool {
        architect: &architect,
        forger: &forger,
    };
    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &steer,
            external_halt: &no_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    assert_eq!(final_run.status, RunStatus::Met);
    // The steering text was recorded on the run and reached a plan prompt.
    assert!(final_run
        .console_inputs
        .iter()
        .any(|c| c.text == "also update the README"));
    let prompts = architect.plan_prompts.lock().unwrap();
    assert!(
        prompts.iter().any(|p| p.contains("also update the README")),
        "no plan prompt carried the steering instruction: {prompts:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// T042 — a permission transmission that times out (blocked too long) is
/// promoted to a whole-run halt: the loop honors the external halt signal and
/// ends `HaltedGuardrail` with `BlockedTimeout`, rather than spinning forever
/// (US2/FR-016). The signal is injected the same way the live gate sets it.
#[tokio::test]
async fn loop_halts_when_external_blocked_timeout_is_signalled() {
    let root = temp_root("blocked");
    let run = Run::new(
        "01J000000000000000000000BL".into(),
        "a goal whose first tool call is never approved".into(),
        vec![],
        "2026-06-13T00:00:00Z".into(),
    );
    let architect = NeverDoneArchitect;
    let forger = ScriptedForger;
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: false } }) };
    // The gate has timed out a transmission before the loop's first check.
    let blocked = std::sync::atomic::AtomicBool::new(true);
    let external_halt = || {
        blocked
            .load(Ordering::SeqCst)
            .then_some(HaltReason::BlockedTimeout)
    };

    let pool = TwoAgentPool {
        architect: &architect,
        forger: &forger,
    };
    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &no_steer,
            external_halt: &external_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    assert_eq!(final_run.status, RunStatus::HaltedGuardrail);
    assert_eq!(final_run.halt_reason, Some(HaltReason::BlockedTimeout));
    // It halted on the external signal before completing any iteration.
    assert_eq!(final_run.iteration, 0);
    let _ = std::fs::remove_dir_all(&root);
}
