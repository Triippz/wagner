//! T018 — Reopen fidelity (FR-105, US1-AS-2, Article VIII).
//!
//! After a run progresses with the window closed, reopening the window folds
//! the host's persisted log and the projection EQUALS the host's live snapshot —
//! no divergence, no loss. The host persists run state each iteration via
//! `state::save`; "reopen" is `state::load` from the same root. This asserts the
//! reloaded projection is byte-identical to the run the host returned live.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use wagner_edge_host::state::{self, ConsoleInput, HaltReason, Run, RunStatus};

struct Lead {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for Lead {
    async fn run(&self, role: Role, _p: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let json = if n >= 1 {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[{"description":"x","agent":"vex","assignment_rationale":"y","may_write_paths":[],"depends_on":[]}],"goal_met_hypothesis":false}"#
                };
                EngineOutcome { signals: vec![], success: true, cost: 0.0, final_text: json.into() }
            }
            Role::Judge => EngineOutcome { signals: vec![], success: true, cost: 0.0, final_text: r#"{"met": true}"#.into() },
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

struct Worker;

#[async_trait]
impl EngineRunner for Worker {
    async fn run(&self, _r: Role, _p: &str) -> EngineOutcome {
        EngineOutcome { signals: vec![], success: true, cost: 0.0, final_text: "ok".into() }
    }
}

struct Pool<'a> {
    lead: &'a dyn EngineRunner,
    worker: &'a dyn EngineRunner,
}

impl AgentPool for Pool<'_> {
    fn lead_id(&self) -> String { "cipher".into() }
    fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
        match id { "cipher" => Some(self.lead), "vex" => Some(self.worker), _ => None }
    }
    fn ids(&self) -> Vec<String> { vec!["cipher".into(), "vex".into()] }
    fn faction(&self, id: &str) -> Faction {
        if id == "vex" { Faction::Forgers } else { Faction::Architects }
    }
    fn name(&self, id: &str) -> String { id.into() }
    fn brief(&self) -> String { "cipher/vex".into() }
}

#[tokio::test]
async fn reopening_folds_the_host_log_to_the_identical_snapshot() {
    let root = std::env::temp_dir().join(format!("wagner-reopen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    let lead = Lead { plan_calls: AtomicUsize::new(0) };
    let worker = Worker;
    let pool = Pool { lead: &lead, worker: &worker };
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };
    let no_steer = Vec::<ConsoleInput>::new;
    let no_halt = || None::<HaltReason>;

    let run_id = "01J0000000000000000REOPEN0".to_string();
    let live = run_goal(
        Run::new(run_id.clone(), "progress then reopen".into(), vec![], "2026-06-16T00:00:00Z".into()),
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &|_| {},
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &|_| {},
            emit_panel: &|_, _| {},
            cancel: None,
        },
    )
    .await;
    assert_eq!(live.status, RunStatus::Met);

    // "Reopen": reload the persisted log from disk (what a reopened window folds).
    let reloaded = state::load(&root, &run_id).expect("host log must reload");

    // The reopened projection equals the live snapshot — no divergence (FR-105).
    assert_eq!(reloaded, live, "reopened projection must equal the live snapshot");
    // And serialization is byte-identical (Article VIII determinism).
    assert_eq!(
        serde_json::to_vec(&reloaded).unwrap(),
        serde_json::to_vec(&live).unwrap(),
    );
    let _ = std::fs::remove_dir_all(&root);
}
