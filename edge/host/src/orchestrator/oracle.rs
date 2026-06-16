//! Oracle planner output parsing (FR-003/FR-004).
//!
//! The Claude Oracle pass returns a JSON plan; the host validates it against
//! `oracle-plan.schema.json` before acting. A non-conforming plan is rejected so
//! the loop can re-prompt once, then raise a Gate transmission.
//!
//! This module is pure parse/validation logic — the actual CLI invocation that
//! produces the text lives in the driver (T026/T027).

use crate::schema::{self, ORACLE_PLAN_SCHEMA};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedSubtask {
    pub description: String,
    /// Roster agent id this subtask is assigned to (validated against the roster).
    pub agent: String,
    pub assignment_rationale: String,
    pub may_write_paths: Vec<String>,
    pub depends_on: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OraclePlan {
    pub schema: String,
    pub subtasks: Vec<PlannedSubtask>,
    pub goal_met_hypothesis: bool,
}

impl OraclePlan {
    pub const SCHEMA: &'static str = "oracle-plan.v2";
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("plan is not valid JSON: {0}")]
    NotJson(String),
    /// Schema-valid JSON that still failed to deserialize into `OraclePlan` — a
    /// shape/type mismatch the schema didn't catch, distinct from `NotJson` so
    /// error matchers don't conflate "not JSON" with "wrong shape".
    #[error("plan JSON did not deserialize into the expected shape: {0}")]
    Shape(String),
    #[error("plan failed schema validation: {0}")]
    Schema(String),
    #[error("plan dependency index {idx} is out of range (only {len} subtasks)")]
    BadDependency { idx: usize, len: usize },
    #[error("plan assigns a subtask to unknown agent `{0}` (not in the roster)")]
    UnknownAgent(String),
}

/// Parse + validate a planner's raw output. Tolerates the CLI wrapping the JSON
/// in prose/markdown by extracting the first balanced top-level JSON object.
/// `roster_ids` are the ids of the hired agents; every subtask must be assigned
/// to one of them (a semantic check the schema can't express).
pub fn parse_plan(raw: &str, roster_ids: &[String]) -> Result<OraclePlan, PlanError> {
    let json_slice = super::json_scan::first_balanced_object(raw).ok_or_else(|| {
        PlanError::NotJson("no top-level JSON object found in planner output".to_string())
    })?;

    let value: serde_json::Value =
        serde_json::from_str(json_slice).map_err(|e| PlanError::NotJson(e.to_string()))?;
    schema::validate(ORACLE_PLAN_SCHEMA, &value).map_err(|e| match e {
        schema::SchemaError::ValidationFailed(m) => PlanError::Schema(m),
        other => PlanError::Schema(other.to_string()),
    })?;

    let plan: OraclePlan =
        serde_json::from_value(value).map_err(|e| PlanError::Shape(e.to_string()))?;

    // Semantic checks the schema can't express.
    let len = plan.subtasks.len();
    for st in &plan.subtasks {
        if !roster_ids.iter().any(|id| id == &st.agent) {
            return Err(PlanError::UnknownAgent(st.agent.clone()));
        }
        for &idx in &st.depends_on {
            if idx >= len {
                return Err(PlanError::BadDependency { idx, len });
            }
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "schema": "oracle-plan.v2",
        "subtasks": [
            {"description":"write the failing test","agent":"cipher","assignment_rationale":"test authoring","may_write_paths":["tests/x.rs"],"depends_on":[]},
            {"description":"implement to pass","agent":"vex","assignment_rationale":"crisp impl","may_write_paths":["src/x.rs"],"depends_on":[0]}
        ],
        "goal_met_hypothesis": false
    }"#;

    fn roster_ids() -> Vec<String> {
        vec!["cipher".to_string(), "vex".to_string()]
    }

    #[test]
    fn parses_a_conforming_plan() {
        let plan = parse_plan(GOOD, &roster_ids()).expect("valid plan");
        assert_eq!(plan.subtasks.len(), 2);
        assert_eq!(plan.subtasks[0].agent, "cipher");
        assert_eq!(plan.subtasks[1].depends_on, vec![0]);
        assert!(!plan.goal_met_hypothesis);
    }

    #[test]
    fn extracts_json_from_prose_and_fences() {
        let wrapped = format!("Here is the plan:\n```json\n{}\n```\nDone.", GOOD);
        assert!(parse_plan(&wrapped, &roster_ids()).is_ok());
    }

    #[test]
    fn rejects_subtask_assigned_to_unknown_agent() {
        // `vex` is replaced with an agent not in the roster — schema passes (it's a
        // non-empty string) but the membership check rejects it.
        let bad = GOOD.replace("\"vex\"", "\"ghost\"");
        assert!(matches!(
            parse_plan(&bad, &roster_ids()),
            Err(PlanError::UnknownAgent(a)) if a == "ghost"
        ));
    }

    #[test]
    fn rejects_empty_agent_id() {
        let bad = GOOD.replace("\"vex\"", "\"\"");
        assert!(matches!(
            parse_plan(&bad, &roster_ids()),
            Err(PlanError::Schema(_))
        ));
    }

    #[test]
    fn rejects_missing_field() {
        // Drop the required `goal_met_hypothesis` field (and its preceding comma).
        let bad = GOOD.replace(",\n        \"goal_met_hypothesis\": false", "");
        assert_ne!(bad, GOOD, "test setup must actually mutate the input");
        assert!(parse_plan(&bad, &roster_ids()).is_err());
    }

    #[test]
    fn rejects_out_of_range_dependency() {
        let bad = GOOD.replace("\"depends_on\":[0]", "\"depends_on\":[7]");
        assert!(matches!(
            parse_plan(&bad, &roster_ids()),
            Err(PlanError::BadDependency { idx: 7, len: 2 })
        ));
    }

    #[test]
    fn rejects_non_json() {
        assert!(matches!(
            parse_plan("the model refused", &roster_ids()),
            Err(PlanError::NotJson(_))
        ));
    }
}
