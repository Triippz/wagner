//! Integration-style unit tests for the voice model download manager.
//!
//! These tests cover status helpers and serialisation only.
//! Download path (download_one, checksum failure, progress ordering) is tested
//! by the inline #[cfg(test)] block inside edge/host/src/voice/models.rs which
//! has access to the private download_one function and a loopback stub.
//! Run with: cargo test -p wagner-edge-host voice

use wagner_edge_host::voice::models::{
    all_models_ready, models_status, ModelProgress, ModelState, ModelsStatus, MODELS,
};

// ── models_status ─────────────────────────────────────────────────────────────

#[test]
fn models_status_all_absent_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let s = models_status(dir.path());
    assert_eq!(s.stt, "absent");
    assert_eq!(s.tts, "absent");
}

#[test]
fn models_status_ready_when_all_present() {
    let dir = tempfile::tempdir().unwrap();
    for m in MODELS {
        std::fs::write(dir.path().join(m.filename), b"placeholder").unwrap();
    }
    let s = models_status(dir.path());
    assert_eq!(s.stt, "ready");
    assert_eq!(s.tts, "ready");
}

#[test]
fn models_status_tts_absent_when_one_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    // stt + tts_model only.
    std::fs::write(dir.path().join(MODELS[0].filename), b"x").unwrap();
    std::fs::write(dir.path().join(MODELS[1].filename), b"x").unwrap();
    let s: ModelsStatus = models_status(dir.path());
    assert_eq!(s.stt, "ready");
    assert_eq!(s.tts, "absent", "tts must be absent when voices file is missing");
}

#[test]
fn all_models_ready_false_on_partial_set() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!all_models_ready(dir.path()));
    for m in MODELS {
        std::fs::write(dir.path().join(m.filename), b"x").unwrap();
    }
    assert!(all_models_ready(dir.path()));
}

// ── ModelState Display ────────────────────────────────────────────────────────

#[test]
fn model_state_display_values() {
    assert_eq!(ModelState::Absent.to_string(), "absent");
    assert_eq!(ModelState::Downloading.to_string(), "downloading");
    assert_eq!(ModelState::Verifying.to_string(), "verifying");
    assert_eq!(ModelState::Ready.to_string(), "ready");
    assert_eq!(ModelState::Failed.to_string(), "failed");
}

// ── ModelProgress serde shape ─────────────────────────────────────────────────

#[test]
fn model_progress_serialises_with_correct_keys() {
    let p = ModelProgress {
        model: "stt".into(),
        state: ModelState::Downloading,
        received: 512,
        total: 1024,
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["model"], "stt");
    assert_eq!(v["state"], "downloading");
    assert_eq!(v["received"], 512);
    assert_eq!(v["total"], 1024);
}
