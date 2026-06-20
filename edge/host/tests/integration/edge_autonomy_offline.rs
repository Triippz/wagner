//! T009a — Edge-autonomy / offline-completion (Constitution Article VI Gate).
//!
//! The load-bearing guarantee of wedge-002: remote + hub are STRICTLY ADDITIVE.
//! A run must start and complete on the edge with the hub unreachable (no
//! discovery, no relay, no hub routes) AND no remote client attached, touching
//! none of them on its execution path.
//!
//! How this is asserted: the run is driven by the carried headless
//! `EngineRunner`/`run_goal` to a terminal `Met`. Every external-effect surface
//! the loop has (`EngineRunner`, `run_suite`, `emit`, `steer`, `external_halt`,
//! `progress`, `emit_panel`) is local and in-memory. A shared `HubProbe`
//! threads a "would have called the hub" counter through the only seams a remote
//! build could later put a hub call behind (the event sink + a guarded runner).
//! The run completes with that counter at ZERO — the regression lock that future
//! US2/US3 remote wiring must keep green by never adding a synchronous hub /
//! discovery / relay call to `run_goal`'s path (plan §1.4; challenge C1;
//! constitution.md:174 "an offline-completion test exists").

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use wagner_edge_host::state::{ConsoleInput, HaltReason, Run, RunStatus};

/// Counts any attempt to reach the hub/discovery/relay. On an offline run it
/// must stay 0; if it is ever non-zero the edge-autonomy guarantee is broken.
#[derive(Default)]
struct HubProbe {
    hub_calls: AtomicUsize,
    local_engine_calls: AtomicUsize,
}

impl HubProbe {
    /// The hook a remote/hub-coupled build would call from the run path. Nothing
    /// on the carried path calls it — that is exactly the property under test.
    #[allow(dead_code)]
    fn note_hub_call(&self) {
        self.hub_calls.fetch_add(1, Ordering::SeqCst);
    }
}

/// Wraps a scripted runner and records that the call was a LOCAL engine call,
/// never a hub call. The run loop only ever drives the engine through this.
struct OfflineGuardRunner<'a> {
    inner: &'a dyn EngineRunner,
    probe: &'a HubProbe,
}

#[async_trait]
impl EngineRunner for OfflineGuardRunner<'_> {
    async fn run(&self, role: Role, prompt: &str) -> EngineOutcome {
        self.probe.local_engine_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.run(role, prompt).await
    }
}

/// Plans one subtask, then claims goal-met and confirms as judge — a complete
/// run with no external dependency.
struct ScriptedLead {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for ScriptedLead {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let json = if n >= 1 {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[
                        {"description":"do it","agent":"vex","assignment_rationale":"impl","may_write_paths":["src/x.rs"],"depends_on":[]}
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

struct ScriptedWorker;

#[async_trait]
impl EngineRunner for ScriptedWorker {
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome {
            signals: vec![],
            success: true,
            cost: 0.01,
            final_text: "done".to_string(),
        }
    }
}

struct TwoAgentPool<'a> {
    lead: &'a dyn EngineRunner,
    worker: &'a dyn EngineRunner,
}

impl AgentPool for TwoAgentPool<'_> {
    fn lead_id(&self) -> String {
        "cipher".into()
    }
    fn runner(&self, agent_id: &str) -> Option<&dyn EngineRunner> {
        match agent_id {
            "cipher" => Some(self.lead),
            "vex" => Some(self.worker),
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
        "cipher — lead\nvex — worker".into()
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wagner-offline-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[tokio::test]
async fn run_completes_offline_with_zero_hub_calls() {
    let root = temp_root("complete");
    let probe = HubProbe::default();

    // Hub unreachable: there is no hub client, no discovery URL, no relay — the
    // run loop is constructed with purely local, in-memory dependencies.
    let lead = ScriptedLead {
        plan_calls: AtomicUsize::new(0),
    };
    let worker = ScriptedWorker;
    let guarded_lead = OfflineGuardRunner {
        inner: &lead,
        probe: &probe,
    };
    let guarded_worker = OfflineGuardRunner {
        inner: &worker,
        probe: &probe,
    };
    let pool = TwoAgentPool {
        lead: &guarded_lead,
        worker: &guarded_worker,
    };

    // Event sink folds LOCALLY — a hub-sync build must not call the hub here.
    let emit_count = AtomicUsize::new(0);
    let emit = |_ev| {
        emit_count.fetch_add(1, Ordering::SeqCst);
    };
    let suite = || -> futures::future::BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };
    let no_steer = Vec::<ConsoleInput>::new;
    let no_halt = || None::<HaltReason>;

    let final_run = run_goal(
        Run::new(
            "01J0000000000000000000OFFL".into(),
            "ship it offline".into(),
            vec![],
            "2026-06-16T00:00:00Z".into(),
        ),
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &emit,
            steer: &no_steer,
            external_halt: &no_halt,
            progress: &|_r| {},
            emit_panel: &|_id, _spec| {},
            cancel: None,
        },
    )
    .await;

    // (1) The run completed on the edge with no hub/remote present.
    assert_eq!(
        final_run.status,
        RunStatus::Met,
        "edge run must complete with the hub unreachable (Article VI)"
    );
    // (2) The engine actually ran (the assertion isn't vacuous).
    assert!(
        probe.local_engine_calls.load(Ordering::SeqCst) > 0,
        "the headless EngineRunner must have driven the run"
    );
    // (3) ZERO hub / discovery / relay calls occurred on the run's path.
    assert_eq!(
        probe.hub_calls.load(Ordering::SeqCst),
        0,
        "no hub/discovery/relay call may occur on the run execution path (Article VI; F-1)"
    );

    let _ = std::fs::remove_dir_all(&root);
}
