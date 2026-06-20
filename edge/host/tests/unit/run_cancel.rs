//! 014 US1 — run cancellation, backpressure-abort, and blocked-timeout halt.
//!
//! T005 — cancel-interrupt: a run cancelled mid-turn discards the in-flight
//!         turn, starts no further turn, and emits a terminal Aborted snapshot.
//! T006 — abort-beats-steer: cancel + a pending steer → run reaches Aborted,
//!         steer is discarded.
//! T010 — blocked-timeout halt: a gate blocked past the timeout promotes the
//!         run to the terminal HaltedGuardrail state.
//! T033 — backpressure-abort: when dispatch returns Backpressure, an authorized
//!         abort still stops the run (abort bypasses the saturated intake).
//!
//! D-TEST-1: scripted runner — no real CLI is spawned. The `BlockingRunner`
//! below simulates a long-running turn by awaiting a oneshot before returning.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures::future::BoxFuture;
use tokio::sync::oneshot;

use wagner_edge_host::bus::{
    AgentRegistry, AllowAll, Bus, Command, DispatchError, Event, NodeId, ParticipantId,
    ParticipantKind, RunCommand, RunEvent, Subscription,
};
use wagner_edge_host::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use wagner_edge_host::orchestrator::judge::SuiteResult;
use wagner_edge_host::orchestrator::run_loop::AgentPool;
use wagner_edge_host::orchestrator::GoalLoopAgent;
use wagner_edge_host::events::{CliSignal, Faction};
use wagner_edge_host::state::{Run, RunStatus};

// ── shared scripted infra ──────────────────────────────────────────────────────

fn participant_id(name: &str) -> ParticipantId {
    ParticipantId {
        node: NodeId("test".into()),
        kind: ParticipantKind::GoalLoop,
        name: name.into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("wagner-rc-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

struct SinglePassArchitect {
    done: AtomicBool,
}

#[async_trait]
impl EngineRunner for SinglePassArchitect {
    async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
        match role {
            Role::Plan => {
                if self.done.load(Ordering::SeqCst) {
                    EngineOutcome { signals: vec![], success: true, cost: 0.0,
                        final_text: r#"{"schema":"oracle-plan.v2","subtasks":[],"goal_met_hypothesis":true}"#.into() }
                } else {
                    self.done.store(true, Ordering::SeqCst);
                    EngineOutcome { signals: vec![], success: true, cost: 0.0,
                        final_text: r#"{"schema":"oracle-plan.v2","subtasks":[
                            {"description":"work","agent":"vex","assignment_rationale":"r","may_write_paths":[],"depends_on":[]}
                        ],"goal_met_hypothesis":false}"#.into() }
                }
            }
            Role::Judge => EngineOutcome { signals: vec![], success: true, cost: 0.0,
                final_text: r#"{"met": true}"#.into() },
            Role::Execute => EngineOutcome::from_signals(vec![CliSignal::Spawned], true),
        }
    }
}

struct ImmediateForger;

#[async_trait]
impl EngineRunner for ImmediateForger {
    async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
        EngineOutcome { signals: vec![], success: true, cost: 0.0, final_text: "done".into() }
    }
}

struct TwoAgentPool<'a> {
    architect: &'a dyn EngineRunner,
    forger: &'a dyn EngineRunner,
}

impl AgentPool for TwoAgentPool<'_> {
    fn lead_id(&self) -> String { "cipher".into() }
    fn runner(&self, agent_id: &str) -> Option<&dyn EngineRunner> {
        match agent_id {
            "cipher" => Some(self.architect),
            "vex" => Some(self.forger),
            _ => None,
        }
    }
    fn ids(&self) -> Vec<String> { vec!["cipher".into(), "vex".into()] }
    fn faction(&self, agent_id: &str) -> Faction {
        if agent_id == "vex" { Faction::Forgers } else { Faction::Architects }
    }
    fn name(&self, agent_id: &str) -> String { agent_id.into() }
    fn brief(&self) -> String { "cipher — architect\nvex — forger".into() }
}

// ── T005 — cancel-interrupt ────────────────────────────────────────────────────

/// A run cancelled mid-turn:
///   • discards the in-flight turn (the forger's pending Execute never completes)
///   • starts no further turn
///   • emits a terminal Aborted snapshot on the bus
#[tokio::test]
async fn cancel_interrupt_discards_inflight_turn_and_emits_aborted() {
    let root = temp_root("t005");
    let bus = Arc::new(Bus::new(256));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    let id = participant_id("goal-loop-t005");
    let mut facts = bus.subscribe(Subscription { topic: "run".into(), filter: None });

    // A forger whose Execute signals "turn entered" then blocks forever — holds a
    // turn in-flight so the test can cancel mid-turn.
    struct BlockingForger {
        entered: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
    }
    #[async_trait]
    impl EngineRunner for BlockingForger {
        async fn run(&self, role: Role, _prompt: &str) -> EngineOutcome {
            if let Role::Execute = role {
                if let Some(tx) = self.entered.lock().await.take() {
                    let _ = tx.send(());
                }
                futures::future::pending::<()>().await; // block forever (in-flight turn)
            }
            EngineOutcome::from_signals(vec![CliSignal::Spawned], true)
        }
    }

    let (entered_tx, entered_rx) = oneshot::channel();
    let architect = SinglePassArchitect { done: AtomicBool::new(false) };
    let forger = BlockingForger { entered: tokio::sync::Mutex::new(Some(entered_tx)) };
    let pool = TwoAgentPool { architect: &architect, forger: &forger };
    let suite = || -> BoxFuture<'static, SuiteResult> {
        Box::pin(async { SuiteResult { passed: true } })
    };

    let run = Run::new(
        "01J000000000000000000000T5".into(),
        "test-cancel-interrupt".into(),
        vec![],
        "2026-06-19T00:00:00Z".into(),
    );

    let ctx = reg.context(id.clone());

    // Canceller: once the turn is in-flight (entered), cancel the run. run_goal
    // select!s on the cancel watch and drops the in-flight turn (FR-013).
    let reg2 = Arc::clone(&reg);
    let canceller = tokio::spawn(async move {
        let _ = entered_rx.await;
        reg2.cancel("run-t005");
    });

    let final_run = reg
        .spawn_run_and_drive(
            "run-t005".to_string(),
            GoalLoopAgent::new(ctx),
            run,
            &pool,
            &root,
            &suite,
        )
        .await;
    let _ = canceller.await;

    assert_eq!(final_run.status, RunStatus::Aborted, "cancelled run must reach Aborted");

    // A terminal Aborted Snapshot must appear on the bus.
    let mut saw_aborted = false;
    while let Some(env) = facts.try_recv() {
        if let Event::Run(RunEvent::Snapshot(r)) = &env.payload {
            if r.status == RunStatus::Aborted {
                saw_aborted = true;
            }
        }
    }
    assert!(saw_aborted, "terminal Aborted snapshot must be published on the bus");

    let _ = std::fs::remove_dir_all(&root);
}

// ── T006 — abort-beats-steer ───────────────────────────────────────────────────

/// When cancel arrives with a pending steer, the run reaches Aborted and the
/// steer is discarded (never applied).
#[tokio::test]
async fn abort_beats_pending_steer() {
    let root = temp_root("t006");
    let bus = Arc::new(Bus::new(256));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    let id = participant_id("goal-loop-t006");
    let mut facts = bus.subscribe(Subscription { topic: "run".into(), filter: None });

    let architect = SinglePassArchitect { done: AtomicBool::new(false) };
    let forger = ImmediateForger;
    let pool = TwoAgentPool { architect: &architect, forger: &forger };
    let suite = || -> BoxFuture<'static, SuiteResult> {
        Box::pin(async { SuiteResult { passed: true } })
    };

    let run = Run::new(
        "01J000000000000000000000T6".into(),
        "test-abort-beats-steer".into(),
        vec![],
        "2026-06-19T00:00:00Z".into(),
    );

    // Queue a steer THEN immediately cancel — abort must win.
    reg.steer("run-t006", "ignored steer".into());
    reg.cancel("run-t006");

    let ctx = reg.context(id.clone());
    let final_run = reg.spawn_run_and_drive(
        "run-t006".to_string(),
        GoalLoopAgent::new(ctx),
        run,
        &pool,
        &root,
        &suite,
    ).await;

    assert_eq!(final_run.status, RunStatus::Aborted, "abort beats steer: run must be Aborted");

    let mut saw_aborted = false;
    while let Some(env) = facts.try_recv() {
        if let Event::Run(RunEvent::Snapshot(r)) = &env.payload {
            if r.status == RunStatus::Aborted { saw_aborted = true; }
        }
    }
    assert!(saw_aborted, "Aborted snapshot published even when steer was pending");

    let _ = std::fs::remove_dir_all(&root);
}

// ── T010 — blocked-timeout halt ───────────────────────────────────────────────

/// A gate that stays blocked past its timeout promotes the run to terminal
/// HaltedGuardrail (halt_reason = blocked_timeout). No cancel involved.
#[tokio::test]
async fn blocked_gate_timeout_halts_run() {
    use wagner_edge_host::orchestrator::run_loop::{run_goal, LoopDeps};
    use wagner_edge_host::state::{HaltReason, RunPhase, RunStatus};

    let root = temp_root("t010");

    // A blocked-past-timeout gate is modelled by external_halt returning
    // BlockedTimeout (FR-016/T042) — the loop halts at the next iteration top.
    // (The wall-clock race the auto-draft used starved the single-threaded timer.)
    let architect = SinglePassArchitect { done: AtomicBool::new(false) };
    let forger = ImmediateForger;
    let pool = TwoAgentPool { architect: &architect, forger: &forger };
    let suite = || -> BoxFuture<'static, SuiteResult> {
        Box::pin(async { SuiteResult { passed: true } })
    };

    let run = Run::new(
        "01J000000000000000000000T0".into(),
        "test-blocked-timeout".into(),
        vec![],
        "2026-06-19T00:00:00Z".into(),
    );

    let halt = || Some(HaltReason::BlockedTimeout);
    let no_emit = |_: wagner_edge_host::events::WagnerEvent| {};
    let no_steer = || Vec::<wagner_edge_host::state::ConsoleInput>::new();
    let no_progress = |_: &Run| {};
    let no_panel = |_: &str, _: serde_json::Value| {};

    let final_run = run_goal(
        run,
        LoopDeps {
            pool: &pool,
            run_suite: &suite,
            runs_root: &root,
            emit: &no_emit,
            steer: &no_steer,
            external_halt: &halt,
            progress: &no_progress,
            emit_panel: &no_panel,
            cancel: None,
        },
    )
    .await;

    assert_eq!(
        final_run.status,
        RunStatus::HaltedGuardrail,
        "blocked-timeout must promote to HaltedGuardrail"
    );
    assert_eq!(final_run.phase, RunPhase::Halted, "phase must be Halted");
    assert_eq!(
        final_run.halt_reason,
        Some(HaltReason::BlockedTimeout),
        "halt_reason must be BlockedTimeout"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ── T033 — backpressure-abort ──────────────────────────────────────────────────

/// When the command intake is saturated (dispatch returns Backpressure), an
/// authorized abort bypasses the full queue and still reaches registry.cancel —
/// so the run stops even when the intake cannot accept more commands.
#[tokio::test]
async fn backpressure_abort_still_stops_run() {
    let root = temp_root("t033");
    let bus = Arc::new(Bus::new(1)); // capacity 1 so saturation is easy
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    let id = participant_id("goal-loop-t033");
    let mut facts = bus.subscribe(Subscription { topic: "run".into(), filter: None });

    let architect = SinglePassArchitect { done: AtomicBool::new(false) };
    let forger = ImmediateForger;
    let pool = TwoAgentPool { architect: &architect, forger: &forger };
    let suite = || -> BoxFuture<'static, SuiteResult> {
        Box::pin(async { SuiteResult { passed: true } })
    };

    let run = Run::new(
        "01J000000000000000000000T3".into(),
        "test-backpressure-abort".into(),
        vec![],
        "2026-06-19T00:00:00Z".into(),
    );

    // Fill the intake so the next dispatch returns Backpressure.
    let _rx = bus.take_commands().expect("receiver");
    bus.dispatch(Command::Run(RunCommand::Start { goal: "filler".into() }), &AllowAll)
        .expect("first command fits");
    let bp = bus.dispatch(
        Command::Run(RunCommand::Start { goal: "overflow".into() }),
        &AllowAll,
    );
    assert!(matches!(bp, Err(DispatchError::Backpressure)), "intake is full");

    // Even with a full intake, abort_run must still cancel the run directly.
    // The registry routes abort around the saturated intake via registry.cancel.
    reg.abort_run("run-t033").expect("abort_run must succeed even under backpressure");

    let ctx = reg.context(id);
    let final_run = reg.spawn_run_and_drive(
        "run-t033".to_string(),
        GoalLoopAgent::new(ctx),
        run,
        &pool,
        &root,
        &suite,
    ).await;

    assert_eq!(final_run.status, RunStatus::Aborted,
        "run must be Aborted even when intake was saturated");

    let mut saw_aborted = false;
    while let Some(env) = facts.try_recv() {
        if let Event::Run(RunEvent::Snapshot(r)) = &env.payload {
            if r.status == RunStatus::Aborted { saw_aborted = true; }
        }
    }
    assert!(saw_aborted, "Aborted snapshot published via direct registry path despite backpressure");

    let _ = std::fs::remove_dir_all(&root);
}
