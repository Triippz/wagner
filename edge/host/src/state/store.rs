//! Run-state persistence: atomic JSON writes (D-RES-1) validated against
//! `run-state.schema.json` before they touch disk (Article VII).

use super::run::{Run, RunStatus};
use crate::schema::{self, RUN_STATE_SCHEMA};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Lightweight per-run summary for the session rail — enough to list and order
/// sessions without loading every full `Run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub name: String,
    pub project_dir: String,
    pub status: RunStatus,
    pub updated_at: String,
    pub goal: String,
}

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

/// List every persisted run under `runs_root` as a summary, newest-first by
/// `updated_at`. Best-effort: a run dir whose `state.json` is missing, corrupt,
/// or schema-invalid is skipped (never fatal) so one bad run can't blank the
/// rail. A missing `runs_root` (no runs yet) yields an empty list, not an error.
pub fn list_summaries(runs_root: &Path) -> Result<Vec<RunSummary>, StoreError> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(runs_root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries.flatten() {
        let run_id = entry.file_name().to_string_lossy().into_owned();
        let run = match load(runs_root, &run_id) {
            Ok(r) => r,
            Err(_) => continue, // skip missing/corrupt/invalid run dirs
        };
        // Legacy runs may have no updated_at — fall back to created_at for order.
        let updated_at = if run.updated_at.is_empty() {
            run.created_at.clone()
        } else {
            run.updated_at.clone()
        };
        out.push(RunSummary {
            run_id: run.run_id,
            name: run.name,
            project_dir: run.project_dir,
            status: run.status,
            updated_at,
            goal: run.goal,
        });
    }
    // Newest-first: updated_at is a normalized RFC3339 `…Z` string, so a plain
    // reverse lexicographic compare is chronological.
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
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

    // T018 / FR-012 / Article IX — privacy boundary. The persisted + syncable unit
    // is the Run (run-state.schema.json, additionalProperties:false); raw voice
    // audio/transcript streams must stay on the local bus and never ride along. A
    // spoken *goal* is fine (it is a goal like any typed one). This locks the data
    // model: adding an audio/transcript field to Run fails here. The full hub-sync
    // guard (D-TEST-4) lands when the host gains a run-sync-to-hub path.
    #[test]
    fn run_state_contract_admits_no_raw_voice_field() {
        // Walk every `properties` map in the schema (top-level + nested `$defs`) so a
        // raw-voice field buried in a sub-object can't slip past the guard.
        fn collect_property_keys(node: &serde_json::Value, out: &mut Vec<String>) {
            match node {
                serde_json::Value::Object(map) => {
                    if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                        out.extend(props.keys().cloned());
                    }
                    map.values().for_each(|v| collect_property_keys(v, out));
                }
                serde_json::Value::Array(items) => {
                    items.iter().for_each(|v| collect_property_keys(v, out));
                }
                _ => {}
            }
        }

        let schema: serde_json::Value = serde_json::from_str(RUN_STATE_SCHEMA).unwrap();
        let mut keys = Vec::new();
        collect_property_keys(&schema, &mut keys);
        assert!(!keys.is_empty(), "run-state schema must expose properties to guard");
        for key in &keys {
            let k = key.to_lowercase();
            assert!(
                !(k.contains("audio") || k.contains("transcript") || k.contains("utterance") || k.contains("pcm")),
                "privacy (FR-012): run-state contract must not carry a raw-voice field, found `{key}`"
            );
        }
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

    #[test]
    fn list_summaries_newest_first_skips_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        for (id, updated) in [
            ("01A0000000000000000000000A", "2026-06-17T03:00:00Z"),
            ("01B0000000000000000000000B", "2026-06-17T01:00:00Z"),
            ("01C0000000000000000000000C", "2026-06-17T02:00:00Z"),
        ] {
            let mut r = sample(id);
            r.updated_at = updated.into();
            r.name = format!("name-{id}");
            save(dir.path(), &r).unwrap();
        }
        // A corrupt run dir must be skipped, not fatal (acceptance E5).
        let bad = run_state_path(dir.path(), "01BAD000000000000000000BAD");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(&bad, b"{ not valid json").unwrap();

        let summaries = list_summaries(dir.path()).unwrap();
        let ids: Vec<_> = summaries.iter().map(|s| s.run_id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "01A0000000000000000000000A",
                "01C0000000000000000000000C",
                "01B0000000000000000000000B"
            ]
        );
    }

    #[test]
    fn list_summaries_empty_when_no_runs_dir() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(list_summaries(&missing).unwrap().is_empty());
    }
}
