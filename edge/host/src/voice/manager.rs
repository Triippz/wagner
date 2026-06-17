//! VoiceManager — app-level voice state (enabled, ready) + router handle.
//!
//! Tauri-free: no tauri types appear here. The shell layer wraps it in
//! `State<'_, VoiceManager>` and drives sidecar lifecycle from there.
//!
//! Thread-safety is provided by an internal `Mutex` so Tauri's `State<'_>`
//! (which requires `Send + Sync`) can hold this directly.

use std::sync::Mutex;

use crate::voice::router::VoiceRouter;

/// Snapshot of voice-feature state returned to the UI.
///
/// Keyed `enabled`/`ready` (no rename) to match the IPC contract the UI lane
/// depends on (`voice_status -> { enabled, ready }`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VoiceStatus {
    pub enabled: bool,
    pub ready: bool,
}

struct Inner {
    enabled: bool,
    ready: bool,
    /// Pre-built router targeting the loopback sidecars. Used by the pipeline
    /// once the sidecars are up (Step 7+). ponytail: expose via a public
    /// accessor when the shell pipeline integration lands.
    #[allow(dead_code)]
    router: VoiceRouter,
}

/// Manages the voice feature toggle and the router reference.
///
/// Default: disabled + not-ready.
pub struct VoiceManager {
    inner: Mutex<Inner>,
}

impl VoiceManager {
    /// Create a new manager: disabled, not ready, router pre-wired to
    /// `http://127.0.0.1:8771` (STT) and `http://127.0.0.1:8772` (TTS).
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                enabled: false,
                ready: false,
                router: VoiceRouter::default_http(
                    "http://127.0.0.1:8771",
                    "http://127.0.0.1:8772",
                ),
            }),
        }
    }

    /// Snapshot of current state.
    pub fn status(&self) -> VoiceStatus {
        let g = self.inner.lock().unwrap();
        VoiceStatus {
            enabled: g.enabled,
            ready: g.ready,
        }
    }

    /// Flip the enabled flag. Does not touch `ready` — the shell layer controls
    /// that after sidecar spawn / health-wait.
    pub fn set_enabled(&self, on: bool) {
        let mut g = self.inner.lock().unwrap();
        g.enabled = on;
        // Disabling always clears ready too (sidecars are stopped).
        if !on {
            g.ready = false;
        }
    }

    /// Mark the sidecars as up (or down). The shell calls this after a
    /// successful health-wait or after a kill.
    pub fn set_ready(&self, ready: bool) {
        let mut g = self.inner.lock().unwrap();
        g.ready = ready;
    }
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RED tests written first — they describe the contract before the implementation.

    #[test]
    fn default_is_disabled_and_not_ready() {
        let vm = VoiceManager::new();
        let s = vm.status();
        assert!(!s.enabled, "new VoiceManager must start disabled");
        assert!(!s.ready, "new VoiceManager must start not-ready");
    }

    #[test]
    fn enabling_flips_enabled_flag() {
        let vm = VoiceManager::new();
        vm.set_enabled(true);
        assert!(vm.status().enabled);
        assert!(!vm.status().ready, "enabling does not auto-set ready");
    }

    #[test]
    fn disabling_clears_both_flags() {
        let vm = VoiceManager::new();
        vm.set_enabled(true);
        vm.set_ready(true);
        vm.set_enabled(false);
        let s = vm.status();
        assert!(!s.enabled);
        assert!(!s.ready, "disabling must clear ready");
    }

    #[test]
    fn set_ready_only_affects_ready_flag() {
        let vm = VoiceManager::new();
        vm.set_enabled(true);
        vm.set_ready(true);
        let s = vm.status();
        assert!(s.enabled);
        assert!(s.ready);
    }

    #[test]
    fn set_ready_false_without_disabling() {
        let vm = VoiceManager::new();
        vm.set_enabled(true);
        vm.set_ready(true);
        vm.set_ready(false);
        let s = vm.status();
        // enabled stays, ready is false (e.g. sidecar crashed)
        assert!(s.enabled);
        assert!(!s.ready);
    }

    #[test]
    fn re_enabling_when_already_enabled_is_idempotent() {
        let vm = VoiceManager::new();
        vm.set_enabled(true);
        vm.set_ready(true);
        // A second set_enabled(true) must not change the existing state.
        vm.set_enabled(true);
        let s = vm.status();
        assert!(s.enabled);
        // ready is unaffected — still true
        assert!(s.ready);
    }
}
