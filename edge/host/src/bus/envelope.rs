//! The `Envelope` — the unit carried by the bus — plus `EventId`, `Timestamp`,
//! `StreamId`, and `Scope` (the multi-tenant seam).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{Contract, Event, ParticipantId, StabilityTier};

/// A unique, sortable event id (ULID), serialized as a Crockford-base32 string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventId(#[schemars(with = "String")] pub Ulid);

/// An RFC 3339 timestamp. Carried as a string at the contract boundary; parsed
/// to a calendar type only where a consumer needs ordering (`011`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Timestamp(pub String);

/// The ordering scope of an event (FR-005). `Run` is the common case; new kinds
/// are added additively.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum StreamId {
    Run(String),
    Agent(String),
    Workspace(String),
}

/// The multi-tenant filter seam (FR-004) — exactly `{user, workspace}` in v1; an
/// org tier or further fields are added additively when multi-tenant lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub user: String,
    pub workspace: String,
}

/// The unit carried by the bus: a typed `Event` payload wrapped with identity,
/// ordering, and scope (FR-001/FR-002). `stream` + monotonic `seq` make
/// per-stream ordering expressible (enforcement is `011` P1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    /// Schema-version id (FR-016) — always `"envelope.v1"` for this version.
    pub schema: String,
    pub id: EventId,
    pub ts: Timestamp,
    pub origin: ParticipantId,
    pub stream: StreamId,
    pub seq: u64,
    pub scope: Scope,
    pub payload: Event,
}

impl Envelope {
    /// Construct an envelope, stamping the current `schema`-version id (FR-016).
    pub fn new(
        id: EventId,
        ts: Timestamp,
        origin: ParticipantId,
        stream: StreamId,
        seq: u64,
        scope: Scope,
        payload: Event,
    ) -> Self {
        Self {
            schema: <Self as Contract>::SCHEMA.to_string(),
            id,
            ts,
            origin,
            stream,
            seq,
            scope,
            payload,
        }
    }
}

impl Contract for Envelope {
    const SCHEMA: &'static str = "envelope.v1";
    const TIER: StabilityTier = StabilityTier::Stable;
}
