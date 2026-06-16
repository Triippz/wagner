//! Guardrails — the three run-halting limits (FR-012): max iterations, cost,
//! and blocked-too-long. Pure logic; the loop calls `check` each iteration and
//! `check_blocked` while a transmission is open.

use crate::state::{Guardrails, HaltReason};

/// A guardrail decision: continue, or halt with a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Continue,
    Halt(HaltReason),
}

/// Evaluate the iteration + cost guardrails at the top of a goal loop.
///
/// `cost_used` is in the unit implied by `guardrails.cost.mode`
/// (CLI-reported usage when available, else wall-clock seconds — FR-015).
pub fn check(guardrails: &Guardrails, cost_used: f64) -> Verdict {
    // An unset iteration cap (`None`) means "run until goal-met" — only cost and
    // the blocked-timeout bound the run then.
    if let Some(max) = guardrails.max_iterations {
        if guardrails.iterations_used >= max {
            return Verdict::Halt(HaltReason::Iterations);
        }
    }
    if let Some(budget) = guardrails.cost.budget {
        if cost_used >= budget {
            return Verdict::Halt(HaltReason::Cost);
        }
    }
    Verdict::Continue
}

/// Evaluate the blocked-too-long guardrail for an open transmission.
/// `blocked_secs` is how long the current transmission has been unanswered.
pub fn check_blocked(guardrails: &Guardrails, blocked_secs: u64) -> Verdict {
    if blocked_secs >= u64::from(guardrails.blocked_timeout_secs) {
        return Verdict::Halt(HaltReason::BlockedTimeout);
    }
    Verdict::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CostBudget, CostMode, Guardrails};

    fn gr(max_iter: Option<u32>, used: u32, budget: Option<f64>) -> Guardrails {
        Guardrails {
            max_iterations: max_iter,
            iterations_used: used,
            blocked_timeout_secs: 1800,
            cost: CostBudget {
                mode: CostMode::CliUsage,
                budget,
                used: 0.0,
            },
        }
    }

    #[test]
    fn continues_under_all_limits() {
        assert_eq!(
            check(&gr(Some(50), 3, Some(1000.0)), 10.0),
            Verdict::Continue
        );
    }

    #[test]
    fn halts_on_max_iterations() {
        assert_eq!(
            check(&gr(Some(50), 50, None), 0.0),
            Verdict::Halt(HaltReason::Iterations)
        );
    }

    #[test]
    fn no_iteration_cap_never_halts_on_iterations() {
        // `None` = run until goal-met: even a huge iteration count keeps going,
        // and cost is the only remaining limit here.
        assert_eq!(check(&gr(None, 1_000_000, None), 0.0), Verdict::Continue);
        // A cost budget still bites with no iteration cap.
        assert_eq!(
            check(&gr(None, 1_000_000, Some(10.0)), 10.0),
            Verdict::Halt(HaltReason::Cost)
        );
    }

    #[test]
    fn halts_on_cost_budget() {
        assert_eq!(
            check(&gr(Some(50), 1, Some(100.0)), 100.0),
            Verdict::Halt(HaltReason::Cost)
        );
    }

    #[test]
    fn no_cost_budget_means_no_cost_halt() {
        // Iterations still bound the run even with no cost budget set.
        assert_eq!(check(&gr(Some(50), 1, None), 1e9), Verdict::Continue);
    }

    #[test]
    fn iterations_checked_before_cost() {
        // Both tripped → iterations wins (checked first, deterministic).
        assert_eq!(
            check(&gr(Some(10), 10, Some(5.0)), 99.0),
            Verdict::Halt(HaltReason::Iterations)
        );
    }

    #[test]
    fn blocked_timeout_trips_at_threshold() {
        assert_eq!(
            check_blocked(&gr(None, 0, None), 1800),
            Verdict::Halt(HaltReason::BlockedTimeout)
        );
        assert_eq!(check_blocked(&gr(None, 0, None), 1799), Verdict::Continue);
    }
}
