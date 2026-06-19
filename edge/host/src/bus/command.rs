//! Core `Command` taxonomy — namespaced, imperative intents (+ `Ext` seam).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Contract, StabilityTier};

/// A namespaced, imperative intent. Adjacently tagged as
/// `{ "type": <namespace>, "data": <leaf> }`, mirroring [`super::Event`]; leaf
/// variants land additively inside each namespace enum (FR-007).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Run(RunCommand),
    Goal(GoalCommand),
    Vault(VaultCommand),
    Voice(VoiceCommand),
    Ui(UiCommand),
    /// Extension intent in a plugin-owned namespace; `payload` validates against
    /// the schema registered for `{ns, name, version}` (FR-009).
    Ext {
        ns: String,
        name: String,
        version: u32,
        payload: serde_json::Value,
    },
}

impl Contract for Command {
    const SCHEMA: &'static str = "command.v1";
    const TIER: StabilityTier = StabilityTier::Stable;
}

// One representative imperative seed per namespace; leaves are derived during
// `011` P0 (the migrated Tauri action handlers + voice intake), added additively.

/// Run-namespace intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunCommand {
    /// `run.start` — begin a run for a goal.
    Start { goal: String },
}

/// Goal-namespace intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalCommand {
    /// `goal.add` — add a goal to the backlog.
    Add { title: String },
}

/// Vault-namespace intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum VaultCommand {
    /// `vault.update_note` — write new contents to a note.
    UpdateNote { path: String, body: String },
}

/// Voice-namespace intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceCommand {
    /// `voice.speak` — synthesize speech for text.
    Speak { text: String },
}

/// UI-namespace intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiCommand {
    /// `ui.focus_surface` — move focus to a workspace surface.
    FocusSurface { surface: String },
}
