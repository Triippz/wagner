//! 011 P4 — the goal loop as a bus participant. Driving `GoalLoopAgent::run`
//! with the deterministic fake `AgentPool` reaches `Met` AND publishes the loop's
//! facts on the bus (a terminal `Snapshot` carrying the met run). Proves the
//! inversion: the loop is a participant publishing facts, not the hub.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use wagner_edge_host::bus::{
    AgentRegistry, Bus, Event, NodeId, ParticipantId, ParticipantKind, RunEvent, StreamId,
    Subscription,
};
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::AgentPool;
use wagner_edge_host::orchestrator::GoalLoopAgent;
use wagner_edge_host::state::{Run, RunStatus};

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
        if agent_id == "vex" { Faction::Forgers } else { Faction::Architects }
    }
    fn name(&self, agent_id: &str) -> String {
        agent_id.into()
    }
    fn brief(&self) -> String {
        "cipher (Cipher) — architect [claude]\nvex (Vex) — forger [codex]".into()
    }
}

/// Plans one subtask on iteration 0, claims goal-met on iteration 1, confirms as judge.
struct ScriptedArchitect {
    plan_calls: AtomicUsize,
}

#[async_trait]
impl EngineRunner for ScriptedArchitect {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                let n = self.plan_calls.fetch_add(1, Ordering::SeqCst);
                let json = if n >= 1 {
                    r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#
                } else {
                    r#"{"schema":"oracle-plan.v2","subtasks":[
                        {"description":"impl","agent":"vex","assignment_rationale":"scoped","may_write_paths":["src/x.rs"],"depends_on":[]}
                    ],"goal_met_hypothesis":false}"#
                };
                EngineOutcome { signals: vec![], success: true, cost: 0.01, final_text: json.into() }
            }
            Role::Judge => {
                EngineOutcome { signals: vec![], success: true, cost: 0.01, final_text: r#"{"met": true}"#.into() }
            }
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

struct ScriptedForger;

#[async_trait]
impl EngineRunner for ScriptedForger {
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome { signals: vec![], success: true, cost: 5.0, final_text: "done".into() }
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wagner-gla-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[tokio::test]
async fn goal_loop_agent_drives_to_met_and_publishes_facts() {
    let root = temp_root("met");
    let bus = Arc::new(Bus::new(256));
    let reg = AgentRegistry::new(Arc::clone(&bus));
    let id = ParticipantId {
        node: NodeId("local".into()),
        kind: ParticipantKind::GoalLoop,
        name: "goal-loop".into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };

    // Observe the facts the participant publishes.
    let mut facts = bus.subscribe(Subscription { topic: "run".into(), filter: None });

    let architect = ScriptedArchitect { plan_calls: AtomicUsize::new(0) };
    let forger = ScriptedForger;
    let pool = TwoAgentPool { architect: &architect, forger: &forger };
    let suite = || -> BoxFuture<'static, SuiteResult> { Box::pin(async { SuiteResult { passed: true } }) };

    let run = Run::new("01J000000000000000000000ME".into(), "build it".into(), vec![], "2026-06-19T00:00:00Z".into());
    let agent = GoalLoopAgent::new(reg.context(id.clone()));
    let final_run = agent.run(run, &pool, &root, &suite).await;

    assert_eq!(final_run.status, RunStatus::Met, "the loop reaches Met with the scripted pool");

    // A terminal Snapshot fact carrying the met run was published on the bus,
    // stamped with the goal-loop participant's identity.
    let mut saw_met_snapshot = false;
    while let Some(env) = facts.try_recv() {
        assert_eq!(env.origin, id, "facts carry the goal-loop identity");
        if let Event::Run(RunEvent::Snapshot(run)) = &env.payload {
            assert!(matches!(env.stream, StreamId::Run(_)));
            if run.status == RunStatus::Met {
                saw_met_snapshot = true;
            }
        }
    }
    assert!(saw_met_snapshot, "a terminal Snapshot fact with status Met reached the bus");

    let _ = std::fs::remove_dir_all(&root);
}
