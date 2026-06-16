//! Engine abstraction — the goal loop orchestrates over `EngineRunner` rather
//! than spawning CLIs directly, so the loop logic is testable with scripted
//! runners (no subscription burned). The real implementation wraps `cli::Driver`.

use crate::events::CliSignal;
use async_trait::async_trait;

/// The role a CLI invocation plays in one loop iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Claude Oracle pass: decompose the goal + propose engine assignment.
    Plan,
    /// Execute a single subtask.
    Execute,
    /// Claude judge pass: confirm the goal is satisfied.
    Judge,
}

/// The result of one engine invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineOutcome {
    pub signals: Vec<CliSignal>,
    pub success: bool,
    /// CLI-reported cost: USD for Claude, token count for Codex (FR-015).
    pub cost: f64,
    /// The final text (the planner's JSON, a judge verdict, or a subtask summary).
    pub final_text: String,
}

impl EngineOutcome {
    /// Pull cost + final text out of a signal stream (what the real driver yields).
    pub fn from_signals(signals: Vec<CliSignal>, success: bool) -> Self {
        let mut cost = 0.0;
        let mut final_text = String::new();
        for s in &signals {
            if let CliSignal::Completed { cost_usd, result } = s {
                if let Some(c) = cost_usd {
                    cost += c;
                }
                if !result.is_empty() {
                    final_text = result.clone();
                }
            }
        }
        Self {
            signals,
            success,
            cost,
            final_text,
        }
    }
}

/// An engine the loop can invoke. Real impl spawns a CLI; test impl scripts results.
#[async_trait]
pub trait EngineRunner: Send + Sync {
    async fn run(&self, role: Role, prompt: &str) -> EngineOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_extracts_cost_and_text_from_signals() {
        let signals = vec![
            CliSignal::Spawned,
            CliSignal::Completed {
                cost_usd: Some(0.42),
                result: "done".into(),
            },
        ];
        let o = EngineOutcome::from_signals(signals, true);
        assert_eq!(o.cost, 0.42);
        assert_eq!(o.final_text, "done");
        assert!(o.success);
    }
}
