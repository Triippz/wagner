//! # Event-bus contracts (spec 013, Phase 0)
//!
//! The public, serializable contract every bus participant authors against: the
//! `Envelope`, the namespaced `Event`/`Command` taxonomy, participant identity +
//! the `Agent` trait, the uniform `PluginManifest`, the closed `Capability`
//! vocabulary, and `StabilityTier`.
//!
//! The contract submodules (`envelope`/`event`/`command`/`participant`/`manifest`)
//! are **pure data**. The one behavioural piece is [`runtime`] — the in-process
//! [`Bus`] (spec `011` P1): a `tokio::broadcast` fan-out that stamps per-stream
//! `seq` and surfaces slow-subscriber lag. Command-intake (`dispatch`) and the
//! registry are still later plan steps (`011` P3/P4).
//!
//! ## Invariants
//! - **Namespaced.** `Event`/`Command` are adjacently-tagged (`{type, data}`)
//!   enums over the v1 namespaces (Run/Goal/Vault/Voice/Ui) + `Ext`. The
//!   namespace set is the *stable structure*; leaf variants land additively
//!   inside each namespace enum without editing the core.
//! - **Plain data.** No `Event`/`Command` embeds a `JoinHandle`, `AppHandle`,
//!   channel sender, or closure (`runtime-architecture.md` §7 seam #1) — every
//!   value is foldable by a pure reducer and recordable.
//! - **Schema-validated.** Every contract type derives [`schemars::JsonSchema`];
//!   the exported draft-2020-12 schemas under `edge/host/schemas/bus/` (every
//!   object `additionalProperties:false`) **ARE the catalog** of what may be
//!   emitted/subscribed, and the single source for the generated TypeScript
//!   bindings (`shared/contracts/`).
//! - **Additive-versioning.** A `stable` type's schema never gains a required
//!   field nor removes/retypes one (FR-017); evolution is optional-field plus a
//!   `version` bump.
//! - **Stability-tiered.** Every type carries a `StabilityTier`; new types
//!   default to `Experimental`, promotion to `Stable` is a deliberate change
//!   that binds the no-break rule.
//!
//! Capabilities are **declared, not enforced** in v1 — the sandbox is deferred
//! (`specs/012` design §13.7).

mod command;
mod dispatch;
mod envelope;
mod event;
mod manifest;
mod participant;
mod runtime;

pub use command::{Command, GoalCommand, RunCommand, UiCommand, VaultCommand, VoiceCommand};
pub use dispatch::{Accepted, AllowAll, CommandAuthorizer, CommandEnvelope, DispatchError};
pub use envelope::{Envelope, EventId, Scope, StreamId, Timestamp};
pub use event::{Event, GoalEvent, RunEvent, UiEvent, VaultEvent, VoiceEvent};
pub use manifest::{Capability, Namespace, PluginManifest, SchemaRef, StabilityTier};
pub use participant::{Agent, AgentError, NodeId, ParticipantId, ParticipantKind, Subscription};
pub use runtime::{Bus, RecvError, Subscriber};

/// Per-type contract metadata: the schema-version id (FR-016) and the stability
/// tier (FR-010). Implemented by the top-level contract types that get an
/// exported schema in the catalog.
pub trait Contract {
    /// Schema-version id, e.g. `"envelope.v1"`.
    const SCHEMA: &'static str;
    /// Stability tier governing the additive-only no-break rule (FR-017).
    const TIER: StabilityTier;
    /// The catalog file stem (the part of [`Self::SCHEMA`] before `.vN`).
    fn schema_name() -> &'static str {
        Self::SCHEMA.split('.').next().unwrap_or(Self::SCHEMA)
    }
}

/// The exported draft-2020-12 JSON Schema catalog — the discoverable set of
/// emittable/subscribable contract types and the single source for the generated
/// TypeScript bindings (FR-019). Each entry is `(file-stem, schema)`; the
/// committed `schemas/bus/<stem>.json` files are diffed against this fresh export
/// by the `bus_schema_validate` drift guard (SC-002), regenerated with
/// `UPDATE_SCHEMAS=1`.
pub fn export_schemas() -> Vec<(String, serde_json::Value)> {
    vec![
        (
            Envelope::schema_name().to_string(),
            serde_json::to_value(schemars::schema_for!(Envelope)).expect("Envelope schema serializes"),
        ),
        (
            Event::schema_name().to_string(),
            serde_json::to_value(schemars::schema_for!(Event)).expect("Event schema serializes"),
        ),
        (
            Command::schema_name().to_string(),
            serde_json::to_value(schemars::schema_for!(Command)).expect("Command schema serializes"),
        ),
        (
            PluginManifest::schema_name().to_string(),
            serde_json::to_value(schemars::schema_for!(PluginManifest))
                .expect("PluginManifest schema serializes"),
        ),
    ]
}
