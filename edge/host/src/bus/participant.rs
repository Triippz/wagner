//! Participant identity (`ParticipantId`/`ParticipantKind`/`NodeId`), the `Agent`
//! trait signature, and `Subscription`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::Envelope;

/// An iroh node identity in its canonical z-base-32 string form. The contract
/// carries the identity as a string; it is parsed to `iroh::NodeId` at the
/// intake boundary (`011` P3), where peer identity is actually authorized.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct NodeId(pub String);

/// Stable identity of a participant (FR-003): which peer/machine, what kind, a
/// stable logical name, and a per-instance id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParticipantId {
    /// The peer/machine identity (iroh node id, z-base-32).
    pub node: NodeId,
    pub kind: ParticipantKind,
    /// A stable logical name (e.g. `"goal-loop"`, `"slack-connector"`).
    pub name: String,
    /// Distinguishes two live instances of the same logical name.
    #[schemars(with = "String")]
    pub instance: Ulid,
}

/// The kind of participant behind a [`ParticipantId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantKind {
    GoalLoop,
    Agent,
    Connector,
    Scheduler,
    Ui,
    System,
}

/// A topic/namespace + optional filter selector (FR-011): e.g. topic `"vault"`
/// filter `Some("*")`, topic `"ext.slack"` filter `Some("message")`, or topic
/// `"stream"` filter `Some("<id>")`. Subscriptions filter by topic/namespace,
/// not by matching a single god-enum. Matching behaviour is `011` P1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Subscription {
    pub topic: String,
    pub filter: Option<String>,
}

/// Lifecycle error returned by an [`Agent`]. Signature-only in this phase; the
/// concrete failure taxonomy lands with participant behaviour (`011` P4).
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent error: {0}")]
    Other(String),
}

/// The single participant contract (FR-013): a logical name, declared
/// subscriptions, and an init/handle/shutdown lifecycle. **Signature only** —
/// implementing, registering, and supervising participants is `011` P4.
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// A stable logical name for this participant.
    fn name(&self) -> &str;
    /// The topics/namespaces this participant subscribes to.
    fn subscriptions(&self) -> Vec<Subscription>;
    /// Called once before the participant receives any envelope.
    async fn init(&mut self) -> Result<(), AgentError> {
        Ok(())
    }
    /// Handle one delivered envelope.
    async fn handle(&mut self, envelope: &Envelope) -> Result<(), AgentError>;
    /// Called once during graceful shutdown.
    async fn shutdown(&mut self) -> Result<(), AgentError> {
        Ok(())
    }
}
