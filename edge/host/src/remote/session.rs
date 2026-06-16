//! Attach / session lifecycle (T029, FR-202/210/212, R-3).
//!
//! A remote client attaches only when the host is ARMED, the requester is
//! VERIFIED (OIDC, ADR-0002), and the requester OWNS the host (same operator).
//! Sessions are ephemeral: a deliberate close requires re-arm; a transient drop
//! while armed re-attaches within the SC-007 window (`< 30 s`, exclusive bound).

use super::arm::ArmState;

/// The SC-007 transient-reattach window. A drop `< 30 s` while armed re-attaches
/// silently; `>= 30 s` requires re-arm (exclusive upper bound).
pub const REATTACH_WINDOW_MS: u64 = 30_000;

/// A remote operator whose OIDC identity has been verified (ADR-0002). Carrying
/// this type is the proof of verification; an unverified peer cannot construct
/// one through the host's normal path.
#[derive(Debug, Clone)]
pub struct VerifiedRemote {
    operator_id: String,
    client_id: String,
}

impl VerifiedRemote {
    pub fn new(operator_id: impl Into<String>, client_id: impl Into<String>) -> Self {
        VerifiedRemote {
            operator_id: operator_id.into(),
            client_id: client_id.into(),
        }
    }

    pub fn operator_id(&self) -> &str {
        &self.operator_id
    }
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

/// Why an attach was refused. Distinct reasons so the client can surface a clear
/// message (never a silent failure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachRefusal {
    /// The host is not armed (no endpoint advertised).
    NotArmed,
    /// The verified requester is not the host owner.
    NotOwner,
    /// The arming ticket has lapsed.
    Expired,
}

/// An established remote session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    operator_id: String,
    client_id: String,
    attached_at_ms: u64,
}

impl Session {
    pub fn operator_id(&self) -> &str {
        &self.operator_id
    }
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn attached_at_ms(&self) -> u64 {
        self.attached_at_ms
    }
}

/// Attempt to attach a verified remote to an armed host at `now_ms`.
pub fn attach(
    arm_state: &ArmState,
    who: &VerifiedRemote,
    now_ms: u64,
) -> Result<Session, AttachRefusal> {
    if !arm_state.armed {
        return Err(AttachRefusal::NotArmed);
    }
    if now_ms >= arm_state.expires_at_ms {
        return Err(AttachRefusal::Expired);
    }
    if who.operator_id() != arm_state.operator_id {
        return Err(AttachRefusal::NotOwner);
    }
    Ok(Session {
        operator_id: who.operator_id().to_string(),
        client_id: who.client_id().to_string(),
        attached_at_ms: now_ms,
    })
}

/// How a session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropKind {
    /// The operator deliberately closed the session — re-arm required.
    Deliberate,
    /// The connection dropped transiently while still armed.
    Transient,
}

/// Whether a dropped session may re-attach WITHOUT re-arming. Only a transient
/// drop within the exclusive `REATTACH_WINDOW_MS` qualifies (SC-007).
pub fn can_reattach(kind: DropKind, elapsed_ms: u64) -> bool {
    matches!(kind, DropKind::Transient) && elapsed_ms < REATTACH_WINDOW_MS
}
