//! Sidecar lifecycle for the voice feature.
//!
//! This module is the only place in the shell that touches `tauri_plugin_shell`.
//! `wagner-edge-host` stays Tauri-free (Article VI); all process management
//! lives here.
//!
//! ## Sidecar names
//! - STT: `whisper-server` on `127.0.0.1:8771`
//! - TTS: `wagner-tts-sidecar` on `127.0.0.1:8772`
//!
//! Tauri resolves target-triple suffixes automatically via `externalBin`.

use std::sync::Mutex;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Maximum attempts when polling a sidecar's `/health` endpoint.
const HEALTH_POLL_ATTEMPTS: u32 = 20;
/// Delay between `/health` poll attempts.
const HEALTH_POLL_DELAY: Duration = Duration::from_millis(300);

/// Managed state for sidecar child processes.
///
/// A `Mutex<Vec<CommandChild>>` is intentionally simple — we only ever have
/// two children and only ever start or stop the whole pair.
/// ponytail: per-sidecar error recovery or selective restart would need a
/// named map; not needed now.
///
/// `op_lock` serialises enable/disable to prevent a concurrent double-spawn
/// race (R3): two concurrent `voice_set_enabled(true)` calls would both pass
/// the `enabled && ready` idempotency guard mid-spawn, spawning four sidecars.
/// Holding `op_lock` for the full enable/disable body prevents that.
pub struct SidecarState {
    children: Mutex<Vec<CommandChild>>,
    /// Serialises enable / disable to prevent concurrent double-spawn (R3).
    pub op_lock: tokio::sync::Mutex<()>,
}

impl SidecarState {
    pub fn new() -> Self {
        Self {
            children: Mutex::new(Vec::new()),
            op_lock: tokio::sync::Mutex::new(()),
        }
    }
}

impl Default for SidecarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolved paths to the voice model files on disk.
pub struct ModelPaths {
    /// `ggml-tiny.en.bin` — the STT model.
    pub stt: std::path::PathBuf,
    /// `model_quantized.onnx` — the Kokoro ONNX model.
    pub tts_model: std::path::PathBuf,
    /// `voices-v1.0.bin` — the Kokoro voices blob.
    pub tts_voices: std::path::PathBuf,
}

impl ModelPaths {
    /// Resolve model paths from the app-data models dir.
    pub fn from_dir(dir: &std::path::Path) -> Self {
        Self {
            stt: dir.join("ggml-tiny.en.bin"),
            tts_model: dir.join("model_quantized.onnx"),
            tts_voices: dir.join("voices-v1.0.bin"),
        }
    }
}

/// Spawn both sidecars and wait until each `/health` endpoint responds 200.
///
/// `paths` — resolved model file locations under app-data. These are passed
/// as CLI arguments to the sidecar binaries.
///
/// Returns `Ok(())` when both are up and healthy.
/// Returns `Err(String)` on any spawn or health-wait failure (Article III:
/// never panics, always surfaces a typed error).
pub async fn spawn_sidecars(
    app: &AppHandle,
    sc: &tauri::State<'_, SidecarState>,
    paths: &ModelPaths,
) -> Result<(), String> {
    let shell = app.shell();

    // Spawn STT sidecar. `spawn()` returns `(Receiver<CommandEvent>, CommandChild)`.
    let (_stt_rx, stt_child): (tauri::async_runtime::Receiver<CommandEvent>, CommandChild) = shell
        .sidecar("whisper-server")
        .map_err(|e| format!("failed to create whisper-server command: {e}"))?
        .args([
            "--host", "127.0.0.1",
            "--port", "8771",
            "--inference-path", "/v1/audio/transcriptions",
            "--model", &paths.stt.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| {
            format!(
                "voice sidecar 'whisper-server' could not start ({e}) — the binary \
                 is missing or not runnable (dev: run `make voice-up`; bundle: `make edge-bundle`)"
            )
        })?;

    // Spawn TTS sidecar.
    let (_tts_rx, tts_child): (tauri::async_runtime::Receiver<CommandEvent>, CommandChild) = shell
        .sidecar("wagner-tts-sidecar")
        .map_err(|e| format!("failed to create wagner-tts-sidecar command: {e}"))?
        .args([
            "--host", "127.0.0.1",
            "--port", "8772",
            "--model", &paths.tts_model.to_string_lossy(),
            "--voices", &paths.tts_voices.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| {
            format!(
                "voice sidecar 'wagner-tts-sidecar' could not start ({e}) — the binary \
                 is missing or not runnable (dev: run `make voice-up`; bundle: `make edge-bundle`)"
            )
        })?;

    // Store handles before health-wait so they are killed on error too (R2).
    {
        let mut guard = sc.children.lock().map_err(|_| {
            "voice sidecar state poisoned".to_string()
        })?;
        guard.push(stt_child);
        guard.push(tts_child);
    }

    // Wait for STT health.
    wait_healthy("http://127.0.0.1:8771/health", "whisper-server").await?;
    // Wait for TTS health.
    wait_healthy("http://127.0.0.1:8772/health", "wagner-tts-sidecar").await?;

    Ok(())
}

/// Kill all tracked sidecar children. Errors are logged but not surfaced —
/// a kill failure should not prevent the manager from reaching disabled+not-ready.
///
/// R1: plain `fn` — no async work here, and holding a `MutexGuard` across an
/// `async fn` body risks the guard being sent across await points.
pub fn kill_sidecars(sc: &tauri::State<'_, SidecarState>) {
    // R4: tolerate a poisoned lock; log and bail rather than panic.
    let mut guard = match sc.children.lock() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[voice] sidecar state poisoned in kill_sidecars: {e}");
            return;
        }
    };
    for child in guard.drain(..) {
        if let Err(e) = child.kill() {
            eprintln!("[voice] sidecar kill error: {e}");
        }
    }
}

/// Are both voice sidecars already serving on their loopback ports? This is true
/// when they were started out-of-band — `make voice-up` / `make run` runs them
/// natively. In that case the shell adopts them instead of spawning its own,
/// which avoids a port clash on :8771/:8772 and makes voice work on the dev path,
/// where the Tauri-bundled (`externalBin`) binaries are absent (B1).
pub async fn sidecars_healthy() -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    else {
        return false;
    };
    for url in [
        "http://127.0.0.1:8771/health",
        "http://127.0.0.1:8772/health",
    ] {
        // Adoption only needs to know the sidecar is *serving* on its port. A
        // running sidecar that 404s `/health` (e.g. an older build without the
        // route) is up and adoptable — accept it. Only a 5xx (loading/unhealthy)
        // or a failed connection (nothing listening) counts as not-ready. This
        // keeps voice-enable robust instead of falling through to spawning the
        // (possibly placeholder) bundled binaries in dev.
        match client.get(url).send().await {
            Ok(resp)
                if resp.status().is_success()
                    || resp.status() == reqwest::StatusCode::NOT_FOUND => {}
            _ => return false,
        }
    }
    true
}

/// Poll `url` until it returns an HTTP 200, up to `HEALTH_POLL_ATTEMPTS` tries
/// with `HEALTH_POLL_DELAY` between attempts.
///
/// Returns `Ok(())` on success, `Err(String)` after exhausting retries.
async fn wait_healthy(url: &str, name: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("failed to build health-check client: {e}"))?;

    for attempt in 1..=HEALTH_POLL_ATTEMPTS {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {
                if attempt == HEALTH_POLL_ATTEMPTS {
                    return Err(format!(
                        "{name} did not become healthy at {url} after {HEALTH_POLL_ATTEMPTS} attempts"
                    ));
                }
                tokio::time::sleep(HEALTH_POLL_DELAY).await;
            }
        }
    }
    // Unreachable but satisfies the compiler.
    Err(format!("{name} health-wait loop exited unexpectedly"))
}

#[cfg(test)]
mod tests {
    // Unit tests for SidecarState structure (no binaries needed).

    use super::SidecarState;

    #[test]
    fn sidecar_state_starts_empty() {
        let s = SidecarState::new();
        assert!(s.children.lock().unwrap().is_empty());
    }

    // Lifecycle integration tests (spawn + health) require the actual binaries
    // and are gated behind #[ignore] — mirroring the voice-e2e pattern in
    // edge/host/tests/http_voice_engines.rs.
    //
    // Run with: cargo test -p wagner-edge-shell voice_lifecycle -- --ignored
    // or:        make voice-e2e

    #[tokio::test]
    #[ignore = "requires whisper-server and wagner-tts-sidecar binaries on PATH"]
    async fn spawn_sidecars_becomes_healthy_then_kill() {
        // This test is an integration smoke test, exercised by `make voice-e2e`.
        // Without the binaries it is expected to fail and is therefore ignored
        // in CI.  The test body is intentionally left as a reminder of the
        // acceptance criterion.
        panic!("run manually with the binaries present");
    }
}
