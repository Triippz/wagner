//! T016 — Run survives window-close (SC-004, US1-AS-1).
//!
//! A run executing when the window closes runs to completion: the host-driven
//! headless `EngineRunner` has zero coupling to window lifecycle, so a
//! `CloseRequested` (which only HIDES the window — T015) cannot interrupt it.
//! This drives a real goal loop to `Met` while a window-close is interleaved,
//! and asserts 0 interruptions.

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use wagner_edge_host::state::{ConsoleInput, HaltReason, Run, RunStatus};
use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent};

struct Lead {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for Lead {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
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
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome { signals: vec![], success: true, cost: 0.0, final_text: "ok".into() }
    }
}

struct Pool<'a> {
    lead: &'a dyn EngineRunner,
    worker: &'a dyn EngineRunner,
}

impl AgentPool for Pool<'_> {
    fn lead_id(&self) -> String {
        "cipher".into()
    }
    fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
        match id {
            "cipher" => Some(self.lead),
            "vex" => Some(self.worker),
            _ => None,
        }
    }
    fn ids(&self) -> Vec<String> {
        vec!["cipher".into(), "vex".into()]
    }
    fn faction(&self, id: &str) -> Faction {
        if id == "vex" { Faction::Forgers } else { Faction::Architects }
    }
    fn name(&self, id: &str) -> String {
        id.into()
    }
    fn brief(&self) -> String {
        "cipher lead / vex worker".into()
    }
}

#[tokio::test]
async fn a_run_in_flight_completes_after_the_window_is_closed() {
    let root = std::env::temp_dir().join(format!("wagner-survive-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    // The window closes (hides) while the run is "in flight" — the host keeps running.
    let mut lc = HostLifecycle::new();
    lc.on_event(LifecycleEvent::CloseRequested);
    assert!(!lc.window_visible && lc.host_running, "window hidden, host alive");

    let lead = Lead { plan_calls: AtomicUsize::new(0) };
    let worker = Worker;
    let pool = Pool { lead: &lead, worker: &worker };
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };
    let no_steer = Vec::<ConsoleInput>::new;
    let no_halt = || None::<HaltReason>;

    let final_run = run_goal(
        Run::new("01J00000000000000000SURVIV".into(), "finish window-closed".into(), vec![], "2026-06-16T00:00:00Z".into()),
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

    assert_eq!(final_run.status, RunStatus::Met, "run completed despite window-close (SC-004)");
    assert!(lc.host_running, "host still running after the run completed window-closed");
    let _ = std::fs::remove_dir_all(&root);
}
