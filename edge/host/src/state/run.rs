//! Run aggregate — the persisted state of one goal-loop execution.
//! Mirrors `schemas/run-state.schema.json`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Drafted,
    Running,
    Met,
    HaltedGuardrail,
    Aborted,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HaltReason {
    Iterations,
    Cost,
    BlockedTimeout,
    /// The run was launched in an unusable state (e.g. no lead-capable agent in
    /// the roster). Surfaced as a terminal halt instead of panicking the loop.
    Misconfigured,
}

/// The fine-grained step the loop is in — surfaced live so the mission bar can
/// show "what's happening right now" beyond the coarse `RunStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    #[default]
    Idle,
    Planning,
    Dispatching,
    Judging,
    Blocked,
    Met,
    Halted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostMode {
    /// Usage reported by the CLI itself (preferred when available).
    CliUsage,
    /// Wall-clock fallback when a CLI exposes no usage signal (FR-015).
    Wallclock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CostBudget {
    pub mode: CostMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
    #[serde(default)]
    pub used: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Guardrails {
    /// Optional runaway-loop cap. `None` = run until goal-met (cost + blocked
    /// timeout remain the brakes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub iterations_used: u32,
    pub blocked_timeout_secs: u32,
    pub cost: CostBudget,
}

impl Guardrails {
    /// R-GUARDRAILS defaults — unbounded iterations (run until goal-met); cost
    /// and the blocked-timeout are the active brakes.
    pub fn defaults() -> Self {
        Self {
            max_iterations: None,
            iterations_used: 0,
            blocked_timeout_secs: 30 * 60,
            cost: CostBudget {
                mode: CostMode::CliUsage,
                budget: None,
                used: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubtaskState {
    Queued,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Subtask {
    pub id: String,
    /// The hired-roster agent id this subtask was dispatched to.
    pub agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignment_rationale: Option<String>,
    pub prompt: String,
    pub state: SubtaskState,
    pub worktree: Option<String>,
    pub result_summary: Option<String>,
    #[serde(default)]
    pub parent_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConsoleInput {
    pub ts: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    pub schema: String,
    pub run_id: String,
    pub goal: String,
    #[serde(default)]
    pub docs: Vec<String>,
    pub status: RunStatus,
    /// Fine-grained live step (mission bar). Not persisted-critical; defaults to Idle.
    #[serde(default)]
    pub phase: RunPhase,
    #[serde(default)]
    pub iteration: u32,
    pub guardrails: Guardrails,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_reason: Option<HaltReason>,
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub transmissions: Vec<String>,
    #[serde(default)]
    pub console_inputs: Vec<ConsoleInput>,
    // --- Session fields (a Run IS a session; see memory/002) ---
    /// The on-disk project directory the session's agents run in. Persisted so a
    /// closed session can be resumed (the pool is rebuilt against this cwd).
    /// Empty on legacy runs created before sessions existed.
    #[serde(default)]
    pub project_dir: String,
    /// Human-readable session label for the rail (defaults to the folder name).
    #[serde(default)]
    pub name: String,
    /// Last-saved timestamp — drives the session rail's newest-first ordering.
    /// Defaults to `created_at` for legacy runs.
    #[serde(default)]
    pub updated_at: String,
    /// The session's goal thread — append-only. Seeded with the first goal at
    /// creation; `add_goal` appends and reactivates the session.
    #[serde(default)]
    pub goals: Vec<String>,
}

impl Run {
    pub const SCHEMA: &'static str = "wagner-run.v1";

    /// Create a fresh drafted run. `run_id` and `created_at` are caller-supplied
    /// (the only non-deterministic values; injected, never generated here — Article VI).
    pub fn new(run_id: String, goal: String, docs: Vec<String>, created_at: String) -> Self {
        Self {
            schema: Self::SCHEMA.to_string(),
            run_id,
            goal: goal.clone(),
            docs,
            status: RunStatus::Drafted,
            phase: RunPhase::Idle,
            iteration: 0,
            guardrails: Guardrails::defaults(),
            created_at: created_at.clone(),
            halt_reason: None,
            subtasks: Vec::new(),
            transmissions: Vec::new(),
            console_inputs: Vec::new(),
            // Session fields. `project_dir`/`name` are set by the caller after
            // construction (start_run/resume_run); `goals` seeds with the first
            // goal so a session is a durable goal-thread from the start;
            // `updated_at` starts equal to `created_at`.
            project_dir: String::new(),
            name: String::new(),
            updated_at: created_at,
            goals: vec![goal],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_seeds_goals_and_updated_at() {
        let r = Run::new(
            "01J0RUN0000000000000000002".into(),
            "first goal".into(),
            vec![],
            "2026-06-17T00:00:00Z".into(),
        );
        assert_eq!(r.goals, vec!["first goal".to_string()]);
        assert_eq!(r.updated_at, "2026-06-17T00:00:00Z");
        assert_eq!(r.project_dir, "");
    }
}
