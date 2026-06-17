//! Run-state persistence: atomic JSON writes (D-RES-1) validated against
//! `run-state.schema.json` before they touch disk (Article VII).

use super::run::Run;
use crate::schema::{self, RUN_STATE_SCHEMA};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(String),
    #[error("schema validation failed: {0}")]
    Schema(#[from] schema::SchemaError),
}

/// The outcome of a store operation (Article V — every op reports its effect).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Created,
    Updated,
}

/// Path to a run's state file under the runs root.
pub fn run_state_path(runs_root: &Path, run_id: &str) -> PathBuf {
    runs_root.join(run_id).join("state.json")
}

/// Persist a run atomically: validate → write to a temp file → fsync → rename.
/// A partial write is never visible to readers (D-RES-1).
pub fn save(runs_root: &Path, run: &Run) -> Result<WriteOutcome, StoreError> {
    let json = schema::validate_serialized(RUN_STATE_SCHEMA, run)?;
    let body = serde_json::to_vec_pretty(&json).map_err(|e| StoreError::Serde(e.to_string()))?;

    let dest = run_state_path(runs_root, &run.run_id);
    let existed = dest.exists();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = dest.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&body)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &dest)?;

    Ok(if existed {
        WriteOutcome::Updated
    } else {
        WriteOutcome::Created
    })
}

/// Load a run, validating it against the schema on read.
pub fn load(runs_root: &Path, run_id: &str) -> Result<Run, StoreError> {
    let path = run_state_path(runs_root, run_id);
    let body = std::fs::read_to_string(&path)?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| StoreError::Serde(e.to_string()))?;
    schema::validate(RUN_STATE_SCHEMA, &value)?;
    serde_json::from_value(value).map_err(|e| StoreError::Serde(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> Run {
        Run::new(
            id.into(),
            "build the thing".into(),
            vec![],
            "2026-06-17T00:00:00Z".into(),
        )
    }

    #[test]
    fn round_trips_session_fields() {
        let dir = tempfile::tempdir().unwrap();
        let mut run = sample("01J0RUN0000000000000000001");
        run.project_dir = "/work/repo".into();
        run.name = "repo".into();
        run.updated_at = "2026-06-17T01:00:00Z".into();
        run.goals = vec!["build the thing".into(), "add tests".into()];
        save(dir.path(), &run).unwrap();

        let loaded = load(dir.path(), &run.run_id).unwrap();
        assert_eq!(loaded.project_dir, "/work/repo");
        assert_eq!(loaded.name, "repo");
        assert_eq!(loaded.updated_at, "2026-06-17T01:00:00Z");
        assert_eq!(
            loaded.goals,
            vec!["build the thing".to_string(), "add tests".to_string()]
        );
    }

    #[test]
    fn loads_legacy_run_without_session_fields() {
        // A run-state JSON written before session fields existed must still load
        // (acceptance E2) — the new fields default rather than failing validation.
        let dir = tempfile::tempdir().unwrap();
        let id = "01J0LEGACY000000000000000";
        let legacy = serde_json::json!({
            "schema": "wagner-run.v1",
            "run_id": id,
            "goal": "legacy goal",
            "status": "paused",
            "guardrails": { "blocked_timeout_secs": 120, "cost": { "mode": "cli_usage" } },
            "created_at": "2026-06-01T00:00:00Z"
        });
        let path = run_state_path(dir.path(), id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let loaded = load(dir.path(), id).unwrap();
        assert_eq!(loaded.goals, Vec::<String>::new());
        assert_eq!(loaded.project_dir, "");
        assert_eq!(loaded.name, "");
    }
}
