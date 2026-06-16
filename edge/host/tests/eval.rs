//! Eval — a deterministic, no-burn regression of the canonical "slugify" goal
//! (see `tests/fixtures/eval/sample-goal.md`). It models a realistic two-faction
//! run with scripted engines and asserts the orchestrator produces the right
//! artifacts: both factions active on the floor, a schema-valid persisted
//! run-state, and a `met` verdict reached via the suite + judge gate.
//!
//! This is the deterministic counterpart to a live run — it never spawns a CLI
//! and never touches a subscription, so it can gate CI on every change.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use wagner_edge_host::events::{CliSignal, WagnerEvent, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use wagner_edge_host::state::{self, ConsoleInput, HaltReason, Run, RunStatus, SubtaskState};

/// Two-agent test pool: lead `cipher` (Claude) + `vex` (Codex).
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

const EVAL_GOAL: &str = "Add a `slugify(text)` utility that lowercases, trims, hyphenates \
    non-alphanumeric runs, and strips edge hyphens; add a unit test; run the suite green.";

fn no_steer() -> Vec<ConsoleInput> {
    Vec::new()
}
fn no_halt() -> Option<HaltReason> {
    None
}

fn no_progress(_r: &Run) {}

fn no_panel(_id: &str, _spec: serde_json::Value) {}

/// The Architect (Claude): on the first plan pass it decomposes the goal into a
/// Forger implementation subtask + an Architect test subtask; on the second pass
/// it hypothesizes goal-met and, as judge, confirms.
struct EvalArchitect {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for EvalArchitect {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let json = if n >= 1 {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[
                        {"description":"implement slugify in the utils module","agent":"vex","assignment_rationale":"scoped implementation work","may_write_paths":["src/utils.rs"],"depends_on":[]},
                        {"description":"write a unit test for slugify (empty, phrase, punctuation)","agent":"cipher","assignment_rationale":"test design is the architect's job","may_write_paths":["tests/utils.rs"],"depends_on":[0]}
                    ],"goal_met_hypothesis":false}"#
                };
                EngineOutcome {
                    signals: vec![],
                    success: true,
                    cost: 0.02,
                    final_text: json.to_string(),
                }
            }
            Role::Judge => EngineOutcome {
                signals: vec![],
                success: true,
                cost: 0.02,
                final_text: r#"{"met": true}"#.to_string(),
            },
            // The architect's test-writing subtask: surfaces an operative on the floor.
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

/// The Forger (Codex): implements the scoped subtask and surfaces on the floor.
struct EvalForger;

#[async_trait]
impl EngineRunner for EvalForger {
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome::from_signals(vec![CliSignal::Spawned], true)
    }
}

fn temp_root() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wagner-eval-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[tokio::test]
async fn eval_slugify_goal_reaches_met_with_both_factions_and_valid_state() {
    let root = temp_root();
    let run_id = "01J0000000000000000000EVAL".to_string();
    let run = Run::new(
        run_id.clone(),
        EVAL_GOAL.into(),
        vec!["README.md".into()],
        "2026-06-13T00:00:00Z".into(),
    );
    let architect = EvalArchitect {
        plan_calls: AtomicUsize::new(0),
    };
    let forger = EvalForger;
    let suite = || SuiteResult { passed: true };
    let events: Mutex<Vec<WagnerEvent>> = Mutex::new(Vec::new());
    let emit = |ev: WagnerEvent| events.lock().unwrap().push(ev);

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
            emit: &emit,
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &no_progress,
            emit_panel: &no_panel,
        },
    )
    .await;

    // 1. The goal is reached via the suite + judge gate.
    assert_eq!(final_run.status, RunStatus::Met);
    assert!(final_run.halt_reason.is_none());

    // 2. Both agents did real work — the plan dispatched one to each.
    assert_eq!(
        final_run.subtasks.len(),
        2,
        "expected two dispatched subtasks"
    );
    assert!(final_run
        .subtasks
        .iter()
        .all(|s| s.state == SubtaskState::Done));
    assert!(final_run.subtasks.iter().any(|s| s.agent_id == "vex"));
    assert!(final_run.subtasks.iter().any(|s| s.agent_id == "cipher"));
    // Each subtask carries its assignment rationale (FR-013 traceability).
    assert!(final_run
        .subtasks
        .iter()
        .all(|s| s.assignment_rationale.is_some()));

    // 3. The floor saw operatives from BOTH factions (US4).
    let ev = events.lock().unwrap();
    assert!(
        ev.iter().any(|e| e.faction == Faction::Architects),
        "no Architect operative reached the floor"
    );
    assert!(
        ev.iter().any(|e| e.faction == Faction::Forgers),
        "no Forger operative reached the floor"
    );

    // 4. The persisted artifact a real run leaves behind is schema-valid.
    let persisted = state::load(&root, &run_id).expect("run-state must be persisted");
    assert_eq!(persisted.status, RunStatus::Met);
    let json = serde_json::to_value(&persisted).unwrap();
    wagner_edge_host::schema::validate(wagner_edge_host::schema::RUN_STATE_SCHEMA, &json)
        .expect("persisted run-state must validate against run-state.schema.json");

    let _ = std::fs::remove_dir_all(&root);
}
