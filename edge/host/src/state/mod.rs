//! Run persistence: the Run aggregate and its atomic, schema-validated store.

pub mod run;
pub mod store;

pub use run::{
    ConsoleInput, CostBudget, CostMode, Guardrails, HaltReason, Run, RunPhase, RunStatus, Subtask,
    SubtaskState,
};
pub use store::{load, run_state_path, save, StoreError, WriteOutcome};
