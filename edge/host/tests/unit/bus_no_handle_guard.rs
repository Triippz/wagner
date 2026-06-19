//! T007 [US1] — the no-handle seam guard (§7 seam #1): `Event`/`Command`/
//! `Envelope` are plain serializable data — a `JoinHandle`/`AppHandle`/channel
//! sender/closure field cannot satisfy `Serialize + DeserializeOwned`, so this
//! file fails to compile if one is ever embedded. Plus a `NoopAgent` proving the
//! `Agent` trait signature is implementable and its `subscriptions()` yields a
//! round-trippable `Subscription` carrying a `vault.*` namespace filter.
//! Covers SC-004, AS-3, FR-011, FR-013, FR-018.

use wagner_edge_host::bus::{Agent, AgentError, Command, Envelope, Event, Subscription};

/// Compiles only for types that are plain serializable data + thread-safe.
fn assert_plain<T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static>() {}

#[test]
fn contract_payloads_are_plain_serializable_data() {
    assert_plain::<Event>();
    assert_plain::<Command>();
    assert_plain::<Envelope>();
}

struct NoopAgent;

#[async_trait::async_trait]
impl Agent for NoopAgent {
    fn name(&self) -> &str {
        "noop"
    }
    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription { topic: "vault".into(), filter: Some("*".into()) }]
    }
    async fn handle(&mut self, _envelope: &Envelope) -> Result<(), AgentError> {
        Ok(())
    }
}

#[test]
fn noop_agent_is_implementable_and_subscriptions_round_trip() {
    let agent = NoopAgent;
    assert_eq!(agent.name(), "noop");

    let subs = agent.subscriptions();
    assert!(
        subs.iter().any(|s| s.topic.starts_with("vault")),
        "FR-011: subscriptions filter by topic/namespace (vault.*)"
    );

    let bytes = serde_json::to_vec(&subs).expect("Subscription serializes");
    let back: Vec<Subscription> = serde_json::from_slice(&bytes).expect("Subscription deserializes");
    assert_eq!(subs, back, "Subscription must round-trip");
}
