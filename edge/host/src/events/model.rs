//! Normalized event model — the single event type the host emits to the frontend.
//! Mirrors `schemas/wagner-event.schema.json` (Article VII).

use serde::{Deserialize, Serialize};

/// The kind of work an operative is doing right now. Drives district + state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Read,
    Edit,
    Test,
    Build,
    Lint,
    Shell,
    Review,
    Diff,
    Judge,
    Plan,
    Decompose,
    Think,
    AwaitPermission,
    AwaitQuestion,
}

/// One of the five zones on the operations floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum District {
    Stacks,
    Forge,
    Mirror,
    Oracle,
    Gate,
}

/// The state ring shown around an operative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperativeState {
    Idle,
    Thinking,
    Working,
    Blocked,
}

/// Which engine an operative belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Faction {
    Architects,
    Forgers,
}

/// The normalized event emitted to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WagnerEvent {
    pub schema: String,
    pub event_id: String,
    pub run_id: String,
    pub operative_id: String,
    /// Display name of the hired agent (floor label). Defaults to the id.
    #[serde(default)]
    pub operative_name: String,
    pub faction: Faction,
    pub activity: Activity,
    pub district: District,
    pub state: OperativeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub handoff_target_operative_id: Option<String>,
    pub ts: String,
}

impl WagnerEvent {
    pub const SCHEMA: &'static str = "wagner-event.v1";
}
