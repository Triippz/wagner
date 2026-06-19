//! The goal loop and its components.
//!
//! - `oracle`    — parse/validate the Claude planner's plan (FR-003/004)
//! - `scheduler` — concurrency cap + worktree-isolation decisions (R-ISOLATION)
//! - `guardrails`— iteration / cost / blocked-timeout limits (FR-012)
//! - `judge`     — goal-met decision: subtasks ∧ suite ∧ Claude confirm (FR-013)
//! - `engine`    — the `EngineRunner` abstraction the loop orchestrates over
//! - `run_loop`  — the autonomous goal loop itself (FR-008/011)

pub mod engine;
pub mod goal_loop_agent;
pub mod guardrails;
pub mod identity;
pub mod json_scan;
pub mod judge;
pub mod oracle;
pub mod panel;
pub mod roster;
pub mod run_loop;
pub mod scheduler;
pub mod workflow;
pub mod workflow_exec;

pub use engine::{EngineOutcome, EngineRunner, Role};
pub use goal_loop_agent::GoalLoopAgent;
pub use guardrails::Verdict;
pub use identity::{scan_catalog, scan_skills, AgentIdentity, SkillRef};
pub use judge::{GoalVerdict, JudgeInputs, SuiteResult};
pub use oracle::{OraclePlan, PlanError, PlannedSubtask};
pub use roster::{Agent, Engine, Roster, RosterError};
pub use run_loop::{run_goal, LoopDeps};
pub use workflow::{
    builtin_templates, EdgeWhen, GateMode, NamedTemplate, StageKind, Workflow, WorkflowEdge,
    WorkflowError, WorkflowNode, WORKFLOW_SCHEMA,
};
pub use workflow_exec::{
    run_workflow, ExecConfig, GateDecision, StepRecord, TestOutcome, WorkflowEnd, WorkflowRun,
    WorkflowRunError,
};
