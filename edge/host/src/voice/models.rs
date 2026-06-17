//! Model download manager for the voice pillar.
//!
//! Tauri-free: no AppHandle here. The shell resolves the models dir under
//! `app.path().app_data_dir()` and passes it in.
//!
//! ## Model set (canonical source: scripts/voice-sidecars.sh)
//!
//! | id         | filename               | port |
//! |------------|------------------------|------|
//! | stt        | ggml-tiny.en.bin       | 8771 |
//! | tts_model  | model_quantized.onnx   | 8772 |
//! | tts_voices | voices-v1.0.bin        | 8772 |
//!
//! ## Progress flow
//!
//! `ModelProgress` events fire in order: `Downloading` → `Verifying` →
//! `Ready` (or `Failed`). The shell forwards these as Tauri events on
//! `wagner://voice-download`.
//!
//! ## Partial-file safety
//!
//! Each model downloads to `<filename>.partial` and is atomically renamed to
//! the final path only after SHA-256 verification passes.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

// ── Model registry ────────────────────────────────────────────────────────────

/// A model entry sourced from scripts/voice-sidecars.sh.
pub struct ModelDef {
    pub id: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    /// Lowercase hex SHA-256 as it appears in voice-sidecars.sh.
    pub sha256: &'static str,
}

// Sourced verbatim from scripts/voice-sidecars.sh — do not change these
// without also updating the shell script.
pub const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "stt",
        filename: "ggml-tiny.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    },
    ModelDef {
        id: "tts_model",
        filename: "model_quantized.onnx",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/onnx/model_quantized.onnx",
        sha256: "fbae9257e1e05ffc727e951ef9b9c98418e6d79f1c9b6b13bd59f5c9028a1478",
    },
    ModelDef {
        id: "tts_voices",
        filename: "voices-v1.0.bin",
        url: "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin",
        sha256: "bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d",
    },
];

// ── ModelState ────────────────────────────────────────────────────────────────

/// Persistent disk state visible to `models_status`. Transient states
/// (`Downloading`, `Verifying`) are not stored — they only appear in live
/// `ModelProgress` callbacks.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelState {
    Absent,
    Downloading,
    Verifying,
    Ready,
    Failed,
}

impl std::fmt::Display for ModelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Absent => "absent",
            Self::Downloading => "downloading",
            Self::Verifying => "verifying",
            Self::Ready => "ready",
            Self::Failed => "failed",
        };
        f.write_str(s)
    }
}

// ── Progress callback ─────────────────────────────────────────────────────────

/// A single progress event emitted by `download_models`.
///
/// Serialised by the shell into the `wagner://voice-download` Tauri event with
/// keys `model` / `state` / `received` / `total` (camelCase intentionally not
/// applied — the UI lane's contract uses these exact names).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelProgress {
    /// Model id (matches `ModelDef::id`): `"stt"` | `"tts_model"` | `"tts_voices"`.
    pub model: String,
    pub state: ModelState,
    /// Bytes received so far.
    pub received: u64,
    /// Total bytes expected (`0` when unknown until download completes).
    pub total: u64,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("HTTP error downloading {model}: {source}")]
    Http {
        model: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("IO error for {model}: {source}")]
    Io {
        model: String,
        #[source]
        source: std::io::Error,
    },
    #[error("SHA-256 mismatch for {model}: expected {expected}, got {actual}")]
    Checksum {
        model: String,
        expected: String,
        actual: String,
    },
}

// ── Status query ──────────────────────────────────────────────────────────────

/// Inspect `dir` to determine whether each model file is present.
///
/// Only `Absent` and `Ready` are reported — transient live states
/// (`Downloading`/`Verifying`) are not meaningful after the process has
/// restarted.
pub fn model_state_on_disk(dir: &Path, def: &ModelDef) -> ModelState {
    let path = dir.join(def.filename);
    if path.exists() {
        ModelState::Ready
    } else {
        ModelState::Absent
    }
}

/// Aggregate disk status for the two user-visible groups: STT and TTS.
///
/// TTS is `Ready` only when both `tts_model` and `tts_voices` are ready.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelsStatus {
    /// Lowercase state string: `"absent"` | `"ready"`.
    pub stt: String,
    /// Lowest state of `tts_model` + `tts_voices`: `"absent"` | `"ready"`.
    pub tts: String,
}

/// Returns `true` when all three model files are present on disk.
pub fn all_models_ready(dir: &Path) -> bool {
    MODELS.iter().all(|m| model_state_on_disk(dir, m) == ModelState::Ready)
}

pub fn models_status(dir: &Path) -> ModelsStatus {
    let stt = model_state_on_disk(dir, &MODELS[0]); // stt
    let tts_model = model_state_on_disk(dir, &MODELS[1]);
    let tts_voices = model_state_on_disk(dir, &MODELS[2]);

    // TTS is ready only when both files are present.
    let tts = if tts_model == ModelState::Ready && tts_voices == ModelState::Ready {
        ModelState::Ready
    } else {
        ModelState::Absent
    };

    ModelsStatus {
        stt: stt.to_string(),
        tts: tts.to_string(),
    }
}

// ── Download ──────────────────────────────────────────────────────────────────

/// Download all models into `dir`, reporting progress via `on_progress`.
///
/// - Each file downloads to `<filename>.partial` then renames on success.
/// - A pre-existing final file with a matching checksum is skipped.
/// - A pre-existing final file with a bad checksum is re-downloaded.
/// - `on_progress` is called synchronously in the async context; do not block.
pub async fn download_models<F>(
    dir: &Path,
    client: &reqwest::Client,
    on_progress: F,
) -> Result<(), ModelError>
where
    F: Fn(ModelProgress),
{
    std::fs::create_dir_all(dir).map_err(|e| ModelError::Io {
        model: "models_dir".into(),
        source: e,
    })?;

    for def in MODELS {
        download_one(dir, client, def, &on_progress).await?;
    }
    Ok(())
}

async fn download_one<F>(
    dir: &Path,
    client: &reqwest::Client,
    def: &ModelDef,
    on_progress: &F,
) -> Result<(), ModelError>
where
    F: Fn(ModelProgress),
{
    let final_path = dir.join(def.filename);
    let partial_path: PathBuf = dir.join(format!("{}.partial", def.filename));

    // Skip if already verified on disk.
    if final_path.exists() {
        if verify_sha256(&final_path, def.sha256).is_ok() {
            on_progress(ModelProgress {
                model: def.id.into(),
                state: ModelState::Ready,
                received: 0,
                total: 0,
            });
            return Ok(());
        }
        // Bad checksum — remove and re-download.
        std::fs::remove_file(&final_path).map_err(|e| ModelError::Io {
            model: def.id.into(),
            source: e,
        })?;
    }
    // Remove any leftover partial from a previous interrupted run.
    let _ = std::fs::remove_file(&partial_path);

    // --- Downloading ---
    on_progress(ModelProgress {
        model: def.id.into(),
        state: ModelState::Downloading,
        received: 0,
        total: 0,
    });

    let resp = client
        .get(def.url)
        .send()
        .await
        .map_err(|e| ModelError::Http {
            model: def.id.into(),
            source: e,
        })?;
    if !resp.status().is_success() {
        return Err(ModelError::Http {
            model: def.id.into(),
            source: resp
                .error_for_status()
                .unwrap_err(),
        });
    }

    let total = resp.content_length().unwrap_or(0);
    let mut received: u64 = 0;

    let mut file = std::fs::File::create(&partial_path).map_err(|e| ModelError::Io {
        model: def.id.into(),
        source: e,
    })?;

    let mut stream = resp;
    loop {
        let chunk = stream.chunk().await.map_err(|e| ModelError::Http {
            model: def.id.into(),
            source: e,
        })?;
        match chunk {
            None => break,
            Some(bytes) => {
                file.write_all(&bytes).map_err(|e| ModelError::Io {
                    model: def.id.into(),
                    source: e,
                })?;
                received += bytes.len() as u64;
                on_progress(ModelProgress {
                    model: def.id.into(),
                    state: ModelState::Downloading,
                    received,
                    total,
                });
            }
        }
    }
    // Flush to disk before verifying.
    file.flush().map_err(|e| ModelError::Io {
        model: def.id.into(),
        source: e,
    })?;
    drop(file);

    // --- Verifying ---
    on_progress(ModelProgress {
        model: def.id.into(),
        state: ModelState::Verifying,
        received,
        total,
    });

    verify_sha256(&partial_path, def.sha256).inspect_err(|_| {
        // Clean up partial on checksum failure so the next run can retry.
        let _ = std::fs::remove_file(&partial_path);
    })?;

    // Atomic rename: partial → final.
    std::fs::rename(&partial_path, &final_path).map_err(|e| ModelError::Io {
        model: def.id.into(),
        source: e,
    })?;

    on_progress(ModelProgress {
        model: def.id.into(),
        state: ModelState::Ready,
        received,
        total,
    });

    Ok(())
}

/// SHA-256 verify `path` against `expected` (lowercase hex).
/// Returns `Ok(())` on match, `Err(ModelError::Checksum)` on mismatch.
fn verify_sha256(path: &Path, expected: &str) -> Result<(), ModelError> {
    let model_id = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let mut file = std::fs::File::open(path).map_err(|e| ModelError::Io {
        model: model_id.clone(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| ModelError::Io {
        model: model_id.clone(),
        source: e,
    })?;
    let actual = format!("{:x}", hasher.finalize());

    if actual == expected {
        Ok(())
    } else {
        Err(ModelError::Checksum {
            model: model_id,
            expected: expected.to_string(),
            actual,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Minimal HTTP/1.1 server that serves a single fixed body then closes.
    /// Returns `(base_url, server_task)`. The task shuts down after
    /// `request_count` requests have been served.
    async fn stub_server(
        body: Vec<u8>,
        request_count: usize,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let url = format!("http://{addr}");
        let task = tokio::spawn(async move {
            for _ in 0..request_count {
                let (mut stream, _) = listener.accept().await.unwrap();
                // Drain the HTTP request headers.
                let mut buf = [0u8; 4096];
                let _n = stream.read(&mut buf).await.unwrap_or(0);
                // Respond with a minimal HTTP/1.1 200.
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(header.as_bytes()).await.unwrap();
                stream.write_all(&body).await.unwrap();
                stream.flush().await.unwrap();
            }
        });
        (url, task)
    }

    /// SHA-256 of the 4-byte payload `[0x01, 0x02, 0x03, 0x04]`.
    fn sha256_of(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    /// Build a client that talks to loopback only (no real network in tests).
    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .build()
            .unwrap()
    }

    // ── models_status ────────────────────────────────────────────────────────

    #[test]
    fn models_status_all_absent_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let status = models_status(dir.path());
        assert_eq!(status.stt, "absent");
        assert_eq!(status.tts, "absent");
    }

    #[test]
    fn models_status_ready_when_all_files_present() {
        let dir = tempfile::tempdir().unwrap();
        for m in MODELS {
            std::fs::write(dir.path().join(m.filename), b"x").unwrap();
        }
        let status = models_status(dir.path());
        assert_eq!(status.stt, "ready");
        assert_eq!(status.tts, "ready");
    }

    #[test]
    fn models_status_tts_absent_when_one_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        // stt + tts_model present but tts_voices absent.
        std::fs::write(dir.path().join(MODELS[0].filename), b"x").unwrap();
        std::fs::write(dir.path().join(MODELS[1].filename), b"x").unwrap();
        let status = models_status(dir.path());
        assert_eq!(status.stt, "ready");
        assert_eq!(status.tts, "absent");
    }

    #[test]
    fn all_models_ready_false_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!all_models_ready(dir.path()));
        for m in MODELS {
            std::fs::write(dir.path().join(m.filename), b"x").unwrap();
        }
        assert!(all_models_ready(dir.path()));
    }

    // ── download_models — happy path ─────────────────────────────────────────

    #[tokio::test]
    async fn download_happy_path_progress_in_order_then_ready() {
        // Serve one stub body per model (3 models).
        let body = vec![0x01u8, 0x02, 0x03, 0x04];
        let expected_sha = sha256_of(&body);

        // Temporarily override MODELS URLs by building a custom model set.
        // We cannot override the global const, so we test download_one directly.
        let dir = tempfile::tempdir().unwrap();
        let (url, _server) = stub_server(body.clone(), 1).await;

        let def = ModelDef {
            id: "test_model",
            filename: "test.bin",
            url: Box::leak(url.into_boxed_str()),
            sha256: Box::leak(expected_sha.clone().into_boxed_str()),
        };

        let client = test_client();
        let events: std::sync::Arc<std::sync::Mutex<Vec<ModelProgress>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        download_one(dir.path(), &client, &def, &|p: ModelProgress| {
            events_clone.lock().unwrap().push(p);
        })
        .await
        .unwrap();

        let evs = events.lock().unwrap();
        // Must see Downloading then Verifying then Ready in order.
        let states: Vec<_> = evs.iter().map(|e| e.state.clone()).collect();
        // Find the sequence: at least one Downloading, then Verifying, then Ready.
        assert!(
            states.contains(&ModelState::Downloading),
            "must emit Downloading"
        );
        assert!(
            states.contains(&ModelState::Verifying),
            "must emit Verifying"
        );
        assert_eq!(states.last(), Some(&ModelState::Ready), "last state must be Ready");

        // Verify ordering: last Downloading comes before Verifying comes before Ready.
        let last_dl = states.iter().rposition(|s| *s == ModelState::Downloading).unwrap();
        let verify_pos = states.iter().position(|s| *s == ModelState::Verifying).unwrap();
        let ready_pos = states.iter().position(|s| *s == ModelState::Ready).unwrap();
        assert!(last_dl < verify_pos, "Downloading must precede Verifying");
        assert!(verify_pos < ready_pos, "Verifying must precede Ready");

        // File must be at final path, not partial.
        assert!(dir.path().join("test.bin").exists());
        assert!(!dir.path().join("test.bin.partial").exists());
    }

    // ── download_models — checksum mismatch ──────────────────────────────────

    #[tokio::test]
    async fn download_checksum_mismatch_leaves_no_partial() {
        let body = vec![0xAAu8; 8];
        let dir = tempfile::tempdir().unwrap();
        let (url, _server) = stub_server(body, 1).await;

        let def = ModelDef {
            id: "bad_model",
            filename: "bad.bin",
            url: Box::leak(url.into_boxed_str()),
            sha256: "0000000000000000000000000000000000000000000000000000000000000000", // wrong
        };

        let client = test_client();
        let events: std::sync::Arc<std::sync::Mutex<Vec<ModelProgress>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let err = download_one(dir.path(), &client, &def, &|p| {
            events_clone.lock().unwrap().push(p);
        })
        .await
        .unwrap_err();

        // Must be a Checksum error.
        assert!(
            matches!(err, ModelError::Checksum { .. }),
            "expected Checksum error, got: {err:?}"
        );
        // Partial file must be cleaned up.
        assert!(
            !dir.path().join("bad.bin.partial").exists(),
            "partial must be deleted on mismatch"
        );
        // Final file must not exist.
        assert!(!dir.path().join("bad.bin").exists());

        // Last progress event must be Verifying (we got there but it failed).
        let evs = events.lock().unwrap();
        let states: Vec<_> = evs.iter().map(|e| e.state.clone()).collect();
        assert!(
            states.contains(&ModelState::Verifying),
            "Verifying must be emitted before the error"
        );
        assert!(
            !states.contains(&ModelState::Ready),
            "Ready must NOT be emitted on checksum failure"
        );
    }

    // ── download_models — re-download on bad cached file ────────────────────

    #[tokio::test]
    async fn download_re_downloads_bad_cached_file() {
        let good_body = vec![0xBBu8; 4];
        let correct_sha = sha256_of(&good_body);
        let dir = tempfile::tempdir().unwrap();

        // Plant a corrupted "final" file.
        std::fs::write(dir.path().join("cached.bin"), b"corrupted").unwrap();

        // The server will serve the good body.
        let (url, _server) = stub_server(good_body.clone(), 1).await;

        let def = ModelDef {
            id: "cached",
            filename: "cached.bin",
            url: Box::leak(url.into_boxed_str()),
            sha256: Box::leak(correct_sha.into_boxed_str()),
        };

        let client = test_client();
        download_one(dir.path(), &client, &def, &|_| {}).await.unwrap();

        // The file must now have the good body.
        let on_disk = std::fs::read(dir.path().join("cached.bin")).unwrap();
        assert_eq!(on_disk, good_body);
    }

    // ── download_models — skip already-verified file ────────────────────────

    #[tokio::test]
    async fn download_skips_already_verified_file() {
        let body = vec![0xCCu8; 4];
        let correct_sha = sha256_of(&body);
        let dir = tempfile::tempdir().unwrap();

        // Plant a correct file upfront — no server needed.
        std::fs::write(dir.path().join("ok.bin"), &body).unwrap();

        // URL points nowhere — if we hit it, the test will fail.
        let def = ModelDef {
            id: "ok",
            filename: "ok.bin",
            url: "http://127.0.0.1:1", // nothing listening here
            sha256: Box::leak(correct_sha.into_boxed_str()),
        };

        let client = test_client();
        let ready_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let rc = ready_count.clone();
        download_one(dir.path(), &client, &def, &move |p| {
            if p.state == ModelState::Ready {
                rc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .await
        .unwrap();

        assert_eq!(
            ready_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "exactly one Ready event for a skipped file"
        );
    }

    // ── partial → rename atomicity ───────────────────────────────────────────

    #[tokio::test]
    async fn partial_file_renamed_to_final_on_success() {
        let body = vec![0xDDu8; 4];
        let correct_sha = sha256_of(&body);
        let dir = tempfile::tempdir().unwrap();
        let (url, _server) = stub_server(body, 1).await;

        let def = ModelDef {
            id: "rename",
            filename: "rename.bin",
            url: Box::leak(url.into_boxed_str()),
            sha256: Box::leak(correct_sha.into_boxed_str()),
        };

        let client = test_client();
        download_one(dir.path(), &client, &def, &|_| {}).await.unwrap();

        assert!(dir.path().join("rename.bin").exists(), "final file must exist");
        assert!(
            !dir.path().join("rename.bin.partial").exists(),
            "partial must be gone"
        );
    }
}
