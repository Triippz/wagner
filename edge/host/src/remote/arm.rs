//! Arming (T022/T028, FR-201, SC-203, R-3).
//!
//! `arm` is the EDGE-ONLY host action that advertises the iroh endpoint,
//! registers the NodeId + signaling ticket, and emits `remote.armed`. Self-arm
//! is made structurally impossible by the type system: `arm` consumes a
//! [`LocalOperator`], whose only constructor is [`LocalOperator::from_host`].
//! There is deliberately no `from_remote`, and no function accepts a remote
//! message to arm — a remote peer carries a `VerifiedRemote` (session.rs), which
//! `arm` does not accept.

use super::RemoteAuditEvent;

/// An operator identity established ON THE HOST. The absence of a remote
/// constructor is the SC-203 guarantee (no self-arm).
#[derive(Debug, Clone)]
pub struct LocalOperator {
    id: String,
}

impl LocalOperator {
    /// Build a local operator identity (host-side only).
    pub fn from_host(id: impl Into<String>) -> Self {
        LocalOperator { id: id.into() }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

/// The host's arming state — armed advertises an endpoint; disarmed does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmState {
    pub armed: bool,
    pub operator_id: String,
    pub node_id: String,
    pub ticket_id: String,
    pub expires_at_ms: u64,
}

impl ArmState {
    /// The not-armed state.
    pub fn disarmed() -> Self {
        ArmState {
            armed: false,
            operator_id: String::new(),
            node_id: String::new(),
            ticket_id: String::new(),
            expires_at_ms: 0,
        }
    }
}

/// Arm (or re-arm) the host. Re-arming replaces in place — a fresh ticket/expiry
/// and a single new `remote.armed` event, no duplicate registration (EC-010).
pub fn arm(
    operator: &LocalOperator,
    node_id: impl Into<String>,
    ticket_id: impl Into<String>,
    now_ms: u64,
    ttl_ms: u64,
) -> (ArmState, RemoteAuditEvent) {
    let node_id = node_id.into();
    let ticket_id = ticket_id.into();
    let state = ArmState {
        armed: true,
        operator_id: operator.id().to_string(),
        node_id: node_id.clone(),
        ticket_id: ticket_id.clone(),
        expires_at_ms: now_ms + ttl_ms,
    };
    let event = RemoteAuditEvent::Armed {
        operator_id: operator.id().to_string(),
        node_id,
        ticket_id,
    };
    (state, event)
}

/// Disarm the host — tears down the advertisement; a re-attach requires re-arm.
pub fn disarm() -> RemoteAuditEvent {
    RemoteAuditEvent::Disarmed
}
