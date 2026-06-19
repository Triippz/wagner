//! Command intake (spec 011 P3) — the single validated, authorized path a
//! [`Command`] takes before it reaches the bus. Collapses the app's ~26 ad-hoc
//! action handlers into one chokepoint: **validate → authorize → stamp →
//! enqueue**. The security + future-tenancy seam, and the "say it / click it /
//! schedule it" unifier (any surface that wants to act builds a `Command` and
//! calls [`super::Bus::dispatch`]).

use super::{Command, EventId};

/// A command accepted by intake, stamped with an id for correlation with the
/// facts it later produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub id: EventId,
}

/// A stamped command enqueued for the registry (011 P4) to route to participants.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEnvelope {
    pub id: EventId,
    pub command: Command,
}

/// Why [`super::Bus::dispatch`] refused a command.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DispatchError {
    /// The (JSON) command failed schema validation at the boundary.
    #[error("command failed schema validation: {0}")]
    Invalid(String),
    /// The authorizer denied the command (Article IX).
    #[error("command denied: {0}")]
    Denied(String),
    /// The bounded command intake is full (the consumer is not keeping up).
    #[error("command intake is full")]
    Backpressure,
    /// No consumer is attached to drain the command intake.
    #[error("no command consumer attached")]
    NoConsumer,
}

/// The authorization seam (Constitution Article IX). v1 ships [`AllowAll`]
/// (single-user); per-capability / per-tenant policy plugs in here later without
/// touching the dispatch path.
pub trait CommandAuthorizer: Send + Sync {
    /// `Ok(())` to accept; `Err(reason)` to deny (becomes [`DispatchError::Denied`]).
    fn authorize(&self, command: &Command) -> Result<(), String>;
}

/// Default v1 policy — accept every command (single operator, full trust).
pub struct AllowAll;

impl CommandAuthorizer for AllowAll {
    fn authorize(&self, _command: &Command) -> Result<(), String> {
        Ok(())
    }
}
