//! Remote sessions (US2/US3). iroh endpoint, edge-only arming, ephemeral
//! attach/session lifecycle, the gate-reuse control seam, and dev-context.
//! Each submodule is implemented test-first in Phases 2/4/5 (ADR-0003).

pub mod arm;
pub mod control;
pub mod devcontext;
pub mod endpoint;
pub mod observability;
pub mod session;

/// The Rust-side mirror of the shared remote-event kinds
/// (`shared/reducer/remote-events.ts`). These are appended to the run log and
/// carry METADATA ONLY — never run-bearing output (F-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteAuditEvent {
    Armed { operator_id: String, node_id: String, ticket_id: String },
    Disarmed,
    Attached { operator_id: String, client_id: String },
    Detached { client_id: String, reason: String },
}

impl RemoteAuditEvent {
    /// The event-kind tag, matching `remote-event.schema.json`'s `kind` enum.
    pub fn kind(&self) -> &'static str {
        match self {
            RemoteAuditEvent::Armed { .. } => "remote.armed",
            RemoteAuditEvent::Disarmed => "remote.disarmed",
            RemoteAuditEvent::Attached { .. } => "remote.attached",
            RemoteAuditEvent::Detached { .. } => "remote.detached",
        }
    }

    /// The operator the event concerns, when it names one.
    pub fn operator_id(&self) -> Option<&str> {
        match self {
            RemoteAuditEvent::Armed { operator_id, .. }
            | RemoteAuditEvent::Attached { operator_id, .. } => Some(operator_id),
            _ => None,
        }
    }
}
