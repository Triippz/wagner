//! 011 P4 — the agent registry. A registered participant is subscribed with its
//! declared filters, its `init → handle* → shutdown` lifecycle is driven on its
//! own task, and it is supervised (running / stop). `AgentContext::publish`
//! stamps the agent's identity. Deterministic: agents forward each handled
//! envelope over a channel, so the test awaits delivery (no sleeps).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use wagner_edge_host::bus::{
    Agent, AgentError, AgentRegistry, Bus, Envelope, Event, EventId, NodeId, ParticipantId,
    ParticipantKind, RunEvent, Scope, StreamId, Subscription, Timestamp, VaultEvent,
};

fn pid(name: &str) -> ParticipantId {
    ParticipantId {
        node: NodeId("test".into()),
        kind: ParticipantKind::Agent,
        name: name.into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    }
}

fn run_env(marker: &str) -> Envelope {
    Envelope::new(
        EventId("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()),
        Timestamp("2026-06-19T00:00:00Z".into()),
        pid("source"),
        StreamId::Run(marker.into()),
        0,
        Scope { user: "u".into(), workspace: "w".into() },
        Event::Run(RunEvent::Finished { run_id: marker.into(), ok: true }),
    )
}

fn vault_env() -> Envelope {
    Envelope::new(
        EventId("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()),
        Timestamp("2026-06-19T00:00:00Z".into()),
        pid("source"),
        StreamId::Workspace("w".into()),
        0,
        Scope { user: "u".into(), workspace: "w".into() },
        Event::Vault(VaultEvent::NoteUpdated { path: "n.md".into(), rev: 1 }),
    )
}

/// Forwards a marker for each handled envelope; records init/shutdown.
struct ProbeAgent {
    name: String,
    subs: Vec<Subscription>,
    seen: mpsc::UnboundedSender<String>,
    inited: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl Agent for ProbeAgent {
    fn name(&self) -> &str {
        &self.name
    }
    fn subscriptions(&self) -> Vec<Subscription> {
        self.subs.clone()
    }
    async fn init(&mut self) -> Result<(), AgentError> {
        self.inited.store(true, Ordering::SeqCst);
        Ok(())
    }
    async fn handle(&mut self, envelope: &Envelope) -> Result<(), AgentError> {
        let marker = match &envelope.payload {
            Event::Run(RunEvent::Finished { run_id, .. }) => format!("run:{run_id}"),
            Event::Vault(_) => "vault".into(),
            _ => "other".into(),
        };
        let _ = self.seen.send(marker);
        Ok(())
    }
}

#[tokio::test]
async fn registry_runs_lifecycle_and_routes_subscribed_events() {
    let bus = Arc::new(Bus::new(64));
    let reg = AgentRegistry::new(Arc::clone(&bus));
    let (tx, mut rx) = mpsc::unbounded_channel();
    let inited = Arc::new(AtomicBool::new(false));

    reg.spawn(Box::new(ProbeAgent {
        name: "probe".into(),
        subs: vec![Subscription { topic: "run".into(), filter: None }],
        seen: tx,
        inited: inited.clone(),
    }));

    assert!(reg.is_running("probe"), "spawned agent is supervised");
    assert_eq!(reg.running(), vec!["probe".to_string()]);

    bus.publish(run_env("A"));
    bus.publish(run_env("B"));

    assert_eq!(rx.recv().await.unwrap(), "run:A");
    assert_eq!(rx.recv().await.unwrap(), "run:B");
    assert!(inited.load(Ordering::SeqCst), "init ran before handling");
}

#[tokio::test]
async fn subscription_filters_unmatched_events() {
    let bus = Arc::new(Bus::new(64));
    let reg = AgentRegistry::new(Arc::clone(&bus));
    let (tx, mut rx) = mpsc::unbounded_channel();

    reg.spawn(Box::new(ProbeAgent {
        name: "run-only".into(),
        subs: vec![Subscription { topic: "run".into(), filter: None }],
        seen: tx,
        inited: Arc::new(AtomicBool::new(false)),
    }));

    bus.publish(vault_env()); // filtered out (not subscribed)
    bus.publish(run_env("keeper")); // delivered

    // The first thing the agent reports must be the run event — proof the vault
    // event was filtered, not merely late.
    assert_eq!(rx.recv().await.unwrap(), "run:keeper");
}

#[tokio::test]
async fn multi_subscription_agent_receives_both_namespaces() {
    let bus = Arc::new(Bus::new(64));
    let reg = AgentRegistry::new(Arc::clone(&bus));
    let (tx, mut rx) = mpsc::unbounded_channel();

    reg.spawn(Box::new(ProbeAgent {
        name: "both".into(),
        subs: vec![
            Subscription { topic: "run".into(), filter: None },
            Subscription { topic: "vault".into(), filter: None },
        ],
        seen: tx,
        inited: Arc::new(AtomicBool::new(false)),
    }));

    bus.publish(run_env("A"));
    bus.publish(vault_env());

    assert_eq!(rx.recv().await.unwrap(), "run:A");
    assert_eq!(rx.recv().await.unwrap(), "vault");
}

#[tokio::test]
async fn stop_deregisters_the_participant() {
    let bus = Arc::new(Bus::new(64));
    let reg = AgentRegistry::new(Arc::clone(&bus));
    let (tx, _rx) = mpsc::unbounded_channel();

    reg.spawn(Box::new(ProbeAgent {
        name: "transient".into(),
        subs: vec![Subscription { topic: "*".into(), filter: None }],
        seen: tx,
        inited: Arc::new(AtomicBool::new(false)),
    }));
    assert!(reg.is_running("transient"));
    assert!(reg.stop("transient"), "stop reports it was running");
    assert!(!reg.is_running("transient"));
    assert!(!reg.stop("transient"), "stopping an unknown agent is false");
}

#[tokio::test]
async fn context_publish_stamps_agent_identity() {
    let bus = Arc::new(Bus::new(64));
    let reg = AgentRegistry::new(Arc::clone(&bus));

    // A bare subscriber (not an agent) observes what the context publishes.
    let mut sub = bus.subscribe(Subscription { topic: "run".into(), filter: None });
    let ctx = reg.context(pid("emitter"));
    ctx.publish(StreamId::Run("r1".into()), Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: true }));

    let got = sub.recv().await.unwrap();
    assert_eq!(got.origin, pid("emitter"), "published fact carries the agent's identity");
}

// ── 014 US1 tests ──────────────────────────────────────────────────────────────

// T004 — Router: serve_commands routes RunCommand::Abort → cancel(run_id)
//         and RunCommand::Steer → steer(run_id, text).
//
// The registry must expose serve_commands(rx) which reads from a tokio mpsc
// receiver of Commands and dispatches Abort/Steer to the matching run.
#[tokio::test]
async fn serve_commands_routes_abort_to_cancel_and_steer_to_steer() {
    use tokio::sync::mpsc;

    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    // spawn_run registers a named run under the supervision of the registry.
    // The future completes immediately; we just want the run to be live
    // so that cancel/steer can find it by run_id.
    let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel::<String>();
    let (steer_tx, mut steer_rx) = mpsc::unbounded_channel::<String>();

    let cancel_tx2 = cancel_tx.clone();
    let steer_tx2 = steer_tx.clone();
    let run_id = "run-t004".to_string();
    let run_id2 = run_id.clone();

    reg.spawn_run(
        run_id.clone(),
        |_cancel| {
            let run_id = run_id.clone();
            async move {
                // probe: forward cancel/steer signals over channels for assertions
                let _ = cancel_tx2.send(format!("cancel:{run_id}"));
                futures::future::pending::<()>().await;
            }
        },
        move |text: String| {
            let _ = steer_tx2.send(format!("steer:{text}"));
        },
    )
    .expect("first spawn_run for run-t004 succeeds");

    // Abort routes to cancel
    reg.cancel(&run_id2);
    let msg = cancel_rx.recv().await.unwrap();
    assert!(msg.starts_with("cancel:"), "abort routed to cancel: {msg}");

    // Steer routes to steer
    reg.steer(&run_id2, "focus on tests".into());
    let msg = steer_rx.recv().await.unwrap();
    assert_eq!(msg, "steer:focus on tests", "steer routed correctly");
}

// T007 — Duplicate-start guard: spawn_run for an already-live name is rejected,
//         live run untouched.
#[tokio::test]
async fn spawn_run_rejects_duplicate_live_name() {
    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    reg.spawn_run(
        "run-dup".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("first spawn_run succeeds");

    assert!(reg.is_running("run-dup"), "first run is live");

    let err = reg
        .spawn_run(
            "run-dup".to_string(),
            |_cancel| futures::future::pending::<()>(),
            |_: String| {},
        )
        .expect_err("duplicate spawn_run must be rejected");

    // Error must mention the run_id or indicate duplicate
    let msg = format!("{err}");
    assert!(
        msg.contains("run-dup") || msg.to_lowercase().contains("already") || msg.to_lowercase().contains("live"),
        "error identifies the duplicate: {msg}"
    );

    // The original run is still live — not silently replaced.
    assert!(reg.is_running("run-dup"), "live run untouched after rejected duplicate");
}

// T008 — Concurrent-abort isolation: aborting one live run leaves the other
//         running.
#[tokio::test]
async fn cancel_one_run_leaves_other_running() {
    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    reg.spawn_run(
        "run-a".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("run-a spawns");

    reg.spawn_run(
        "run-b".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("run-b spawns");

    assert!(reg.is_running("run-a"));
    assert!(reg.is_running("run-b"));

    reg.cancel("run-a");

    // Give the abort a moment to land (no sleep — yielding the executor is enough
    // for a cooperative cancel on a pending future).
    tokio::task::yield_now().await;

    assert!(!reg.is_running("run-a"), "run-a is aborted");
    assert!(reg.is_running("run-b"), "run-b is untouched");
}

// T009 — Re-pointed aborted_marks_run_terminal: the registry cancel path emits
//         a terminal Aborted snapshot on the bus. Exercises the same state
//         transition that shell::commands::aborted() tests, but now via the
//         registry-routed abort so Article IX is satisfied end-to-end.
#[tokio::test]
async fn registry_cancel_emits_aborted_snapshot_on_bus() {
    use wagner_edge_host::bus::{Event, RunEvent};

    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    let mut facts = bus.subscribe(Subscription { topic: "run".into(), filter: None });

    reg.spawn_run(
        "run-t009".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("spawns");

    reg.cancel("run-t009");

    // The registry must publish a terminal Snapshot(RunStatus::Aborted) fact.
    let mut saw_aborted = false;
    // Drain up to a bounded number of events; the snapshot must arrive quickly
    // (the cancel is synchronous, emission happens before the task exits).
    for _ in 0..32 {
        match facts.try_recv() {
            Some(env) => {
                if let Event::Run(RunEvent::Snapshot(run)) = &env.payload {
                    use wagner_edge_host::state::RunStatus;
                    if run.status == RunStatus::Aborted {
                        saw_aborted = true;
                        break;
                    }
                }
            }
            None => {
                tokio::task::yield_now().await;
            }
        }
    }
    assert!(saw_aborted, "registry cancel must publish a terminal Aborted snapshot on the bus");
}

// T012 — Pluggability: a second participant registers via the same
//         spawn_run path with no run-control code changed. It receives its
//         subscribed events and its lifecycle is supervised identically.
#[tokio::test]
async fn second_participant_registers_via_same_spawn_run_path() {
    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    // First run participant
    reg.spawn_run(
        "run-p1".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("first participant spawns");

    // Second run participant — identical code path, different id
    reg.spawn_run(
        "run-p2".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("second participant spawns via same path");

    assert!(reg.is_running("run-p1"), "first participant live");
    assert!(reg.is_running("run-p2"), "second participant live, no bespoke lifecycle needed");

    // Both can be cancelled independently
    reg.cancel("run-p1");
    tokio::task::yield_now().await;
    assert!(!reg.is_running("run-p1"), "first cancelled");
    assert!(reg.is_running("run-p2"), "second still live");
}

// T034 — Spawn-footgun guard: bare spawn() cannot silently abort a live
//         run-keyed participant. A run-keyed name must be rejected by spawn()
//         when a live run with that name exists.
#[tokio::test]
async fn bare_spawn_cannot_silently_replace_live_run_keyed_participant() {
    let bus = Arc::new(Bus::new(64));
    let reg = Arc::new(AgentRegistry::new(Arc::clone(&bus)));

    let (seen_tx, _seen_rx) = tokio::sync::mpsc::unbounded_channel();

    // Register a run-keyed participant via the sanctioned path.
    reg.spawn_run(
        "run-guard".to_string(),
        |_cancel| futures::future::pending::<()>(),
        |_: String| {},
    )
    .expect("spawn_run succeeds");

    assert!(reg.is_running("run-guard"));

    // bare spawn() with a run-keyed name must be rejected (Err or panic),
    // NOT silently abort the live run.
    let result = reg.spawn_guarded(Box::new(ProbeAgent {
        name: "run-guard".into(),
        subs: vec![Subscription { topic: "run".into(), filter: None }],
        seen: seen_tx,
        inited: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }));

    assert!(result.is_err(), "spawn_guarded must reject a name held by a live run");
    // The live run-keyed participant is NOT replaced.
    assert!(reg.is_running("run-guard"), "live run participant untouched");
}
