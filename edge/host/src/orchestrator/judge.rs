//! Goal-met judgment (FR-013).
//!
//! A goal is met only when ALL three hold:
//!   (a) every planned subtask is complete,
//!   (b) the project's full automated test suite passes,
//!   (c) the Claude judge pass confirms the work satisfies the goal.
//!
//! The suite runner is injectable so the verdict logic is testable without
//! actually shelling out.

/// Outcome of running the project's test suite (cargo + vitest + playwright).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuiteResult {
    pub passed: bool,
}

/// The three inputs to the goal-met decision.
#[derive(Debug, Clone, Copy)]
pub struct JudgeInputs {
    pub all_subtasks_done: bool,
    pub suite: SuiteResult,
    pub claude_confirms: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalVerdict {
    Met,
    /// Not met; carries the first unmet criterion for the run log / bubble.
    NotMet(&'static str),
}

/// FR-013: met = a ∧ b ∧ c. Reports the first failing criterion deterministically.
pub fn decide(inputs: JudgeInputs) -> GoalVerdict {
    if !inputs.all_subtasks_done {
        return GoalVerdict::NotMet("subtasks incomplete");
    }
    if !inputs.suite.passed {
        return GoalVerdict::NotMet("test suite failing");
    }
    if !inputs.claude_confirms {
        return GoalVerdict::NotMet("judge not satisfied");
    }
    GoalVerdict::Met
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(a: bool, b: bool, c: bool) -> JudgeInputs {
        JudgeInputs {
            all_subtasks_done: a,
            suite: SuiteResult { passed: b },
            claude_confirms: c,
        }
    }

    #[test]
    fn all_three_true_is_met() {
        assert_eq!(decide(inputs(true, true, true)), GoalVerdict::Met);
    }

    #[test]
    fn incomplete_subtasks_blocks_first() {
        assert_eq!(
            decide(inputs(false, false, false)),
            GoalVerdict::NotMet("subtasks incomplete")
        );
    }

    #[test]
    fn failing_suite_blocks_even_if_judge_would_confirm() {
        assert_eq!(
            decide(inputs(true, false, true)),
            GoalVerdict::NotMet("test suite failing")
        );
    }

    #[test]
    fn judge_must_confirm_even_with_green_suite() {
        assert_eq!(
            decide(inputs(true, true, false)),
            GoalVerdict::NotMet("judge not satisfied")
        );
    }
}
