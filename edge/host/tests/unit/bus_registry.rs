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
