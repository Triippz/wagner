//! Slack connector participant (spec 011 P6) — the first external integration,
//! proving the connector shape: subscribe to intents (`ext.slack` `send` events),
//! call the external API behind an injectable [`SlackTransport`], retry on
//! failure (the connector owns its retry — a flaky API never blocks the bus), and
//! publish a fact (`ext.slack` `message_sent`) on success.

use std::sync::Arc;

use async_trait::async_trait;

use crate::bus::{Agent, AgentContext, AgentError, Envelope, Event, StreamId, Subscription};

/// The external Slack call, injected so the connector is testable without a real
/// workspace (a fake transport scripts success/failure).
#[async_trait]
pub trait SlackTransport: Send + Sync {
    /// Post `text` to `channel`; returns the message id/ts on success.
    async fn post_message(&self, channel: &str, text: &str) -> Result<String, String>;
}

/// A Slack connector: turns `ext.slack` `send` intents into posted messages and
/// publishes a `message_sent` fact. Owns its own bounded retry.
pub struct SlackConnector {
    ctx: AgentContext,
    transport: Arc<dyn SlackTransport>,
    max_retries: u32,
}

impl SlackConnector {
    pub fn new(ctx: AgentContext, transport: Arc<dyn SlackTransport>, max_retries: u32) -> Self {
        Self { ctx, transport, max_retries }
    }

    /// Post a message, retrying up to `max_retries` times; on success publish a
    /// `message_sent` fact and return `Ok`. Returns `Err` once retries are
    /// exhausted — the failure is the connector's own concern, the bus is never
    /// blocked by it.
    pub async fn send_message(&self, channel: &str, text: &str) -> Result<(), String> {
        let mut attempt = 0;
        loop {
            match self.transport.post_message(channel, text).await {
                Ok(id) => {
                    self.ctx.publish(
                        StreamId::Workspace("slack".into()),
                        Event::Ext {
                            ns: "slack".into(),
                            name: "message_sent".into(),
                            version: 1,
                            payload: serde_json::json!({ "channel": channel, "ts": id }),
                        },
                    );
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt > self.max_retries {
                        return Err(e);
                    }
                    // ponytail: immediate retry; add exponential backoff + a dead-
                    // letter queue when a real flaky workspace needs it.
                }
            }
        }
    }
}

#[async_trait]
impl Agent for SlackConnector {
    fn name(&self) -> &str {
        "slack-connector"
    }
    fn subscriptions(&self) -> Vec<Subscription> {
        // Slack-namespace intents only.
        vec![Subscription { topic: "ext.slack".into(), filter: Some("send".into()) }]
    }
    async fn handle(&mut self, envelope: &Envelope) -> Result<(), AgentError> {
        if let Event::Ext { ns, name, payload, .. } = &envelope.payload {
            if ns == "slack" && name == "send" {
                let channel = payload.get("channel").and_then(|v| v.as_str()).unwrap_or_default();
                let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or_default();
                self.send_message(channel, text)
                    .await
                    .map_err(AgentError::Other)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{AgentRegistry, Bus, NodeId, ParticipantId, ParticipantKind};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn ctx(bus: &Arc<Bus>) -> AgentContext {
        AgentRegistry::new(Arc::clone(bus)).context(ParticipantId {
            node: NodeId("local".into()),
            kind: ParticipantKind::Connector,
            name: "slack-connector".into(),
            instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        })
    }

    /// Fails the first `fail_times` calls, then succeeds.
    struct FlakyTransport {
        fail_times: u32,
        calls: AtomicU32,
    }
    #[async_trait]
    impl SlackTransport for FlakyTransport {
        async fn post_message(&self, _channel: &str, _text: &str) -> Result<String, String> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fail_times {
                Err(format!("503 (attempt {n})"))
            } else {
                Ok("1700000000.000100".into())
            }
        }
    }

    #[tokio::test]
    async fn retries_then_publishes_a_fact() {
        let bus = Arc::new(Bus::new(16));
        let mut facts = bus.subscribe(Subscription { topic: "ext.slack".into(), filter: None });
        let transport = Arc::new(FlakyTransport { fail_times: 2, calls: AtomicU32::new(0) });
        let connector = SlackConnector::new(ctx(&bus), transport.clone(), 3);

        connector.send_message("#wagner", "Friday report ready").await.expect("succeeds after retries");

        assert_eq!(transport.calls.load(Ordering::SeqCst), 3, "two failures + one success");
        let fact = facts.recv().await.expect("fact published");
        match fact.payload {
            Event::Ext { ns, name, payload, .. } => {
                assert_eq!(ns, "slack");
                assert_eq!(name, "message_sent");
                assert_eq!(payload["channel"], "#wagner");
            }
            other => panic!("expected an ext.slack fact, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn exhausted_retries_error_without_blocking_the_bus() {
        let bus = Arc::new(Bus::new(16));
        let mut facts = bus.subscribe(Subscription { topic: "ext.slack".into(), filter: None });
        let transport = Arc::new(FlakyTransport { fail_times: u32::MAX, calls: AtomicU32::new(0) });
        let connector = SlackConnector::new(ctx(&bus), transport.clone(), 2);

        let result = connector.send_message("#wagner", "nope").await;
        assert!(result.is_err(), "errors once retries are exhausted");
        assert_eq!(transport.calls.load(Ordering::SeqCst), 3, "initial + 2 retries");
        // The bus was never blocked: no fact, and the call returned promptly.
        assert!(facts.try_recv().is_none(), "no fact on exhausted failure");
    }
}
