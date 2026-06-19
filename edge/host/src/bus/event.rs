//! Core `Event` taxonomy — namespaced, past-tense facts (+ `Ext` seam).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Contract, StabilityTier};

/// A namespaced, past-tense fact carried by the bus. Adjacently tagged as
/// `{ "type": <namespace>, "data": <leaf> }`. The namespace set is the **stable
/// structure**; leaf variants land additively inside each namespace enum without
/// editing this core (FR-006).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Run(RunEvent),
    Goal(GoalEvent),
    Vault(VaultEvent),
    Voice(VoiceEvent),
    Ui(UiEvent),
    /// Extension fact in a plugin-owned namespace; `payload` validates against
    /// the schema registered for `{ns, name, version}` (FR-009).
    Ext {
        ns: String,
        name: String,
        version: u32,
        payload: serde_json::Value,
    },
}

impl Contract for Event {
    const SCHEMA: &'static str = "event.v1";
    const TIER: StabilityTier = StabilityTier::Stable;
}

/// The compile-time registry of extension-payload schemas (FR-019: in v1 the
/// catalog is a compile-time artifact; a load-time, config-discovered registry
/// lands additively when third-party plugins do). Keyed by `(ns, name, version)`.
const EXT_SCHEMAS: &[(&str, &str, u32, &str)] = &[(
    "slack",
    "message",
    1,
    include_str!("../../schemas/bus/ext/ext-slack-message.schema.json"),
)];

impl Event {
    /// Resolve the registered JSON Schema source for an extension fact by its
    /// `(ns, name, version)` from the compile-time catalog (FR-019). `None` when
    /// no schema is registered for that triple.
    pub fn ext_schema(ns: &str, name: &str, version: u32) -> Option<&'static str> {
        EXT_SCHEMAS
            .iter()
            .find(|(n, m, v, _)| *n == ns && *m == name && *v == version)
            .map(|&(_, _, _, src)| src)
    }

    /// Validate an `Event::Ext` fact's `payload` against its registered schema
    /// (FR-009, EC-005). The core `Event` enum is **not** edited to add an
    /// extension type — the `Ext` seam resolves the schema from the catalog. A
    /// non-`Ext` event is validated against the core schema elsewhere, so this is
    /// a no-op for it.
    pub fn validate_ext(&self) -> Result<(), crate::schema::SchemaError> {
        let Event::Ext { ns, name, version, payload } = self else {
            return Ok(());
        };
        let schema = Self::ext_schema(ns, name, *version).ok_or_else(|| {
            crate::schema::SchemaError::InvalidSchema(format!(
                "no schema registered for ext {ns}.{name}.v{version}"
            ))
        })?;
        crate::schema::validate(schema, payload)
    }
}

// One representative seed leaf per namespace. The full leaf set is derived during
// `011` P0 (the 7 `wagner://*` channels + voice) and added additively (FR-006).

/// Run-namespace facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunEvent {
    /// `run.finished` — a run reached a terminal state.
    Finished { run_id: String, ok: bool },
}

/// Goal-namespace facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalEvent {
    /// `goal.added` — a goal entered the backlog.
    Added { goal_id: String, title: String },
}

/// Vault-namespace facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum VaultEvent {
    /// `vault.note_updated` — a note's contents changed.
    NoteUpdated { path: String, rev: u64 },
}

/// Voice-namespace facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceEvent {
    /// `voice.utterance_transcribed` — STT produced text for an utterance.
    UtteranceTranscribed { text: String },
}

/// UI-namespace facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiEvent {
    /// `ui.surface_focused` — a workspace surface gained focus.
    SurfaceFocused { surface: String },
}
