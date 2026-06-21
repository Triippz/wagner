//! Acoustic echo cancellation (spec 015, FR-011a) — wraps the WebRTC APM AEC3 via
//! `webrtc-audio-processing` (bundled/static, BSD-3-Clause; research R5). Removes the
//! app's own TTS playback (the render reference) from the mic capture so the
//! always-on wake detector and the spoken-cancel recognizer don't self-trigger.
//!
//! Frame cadence: the APM processes **10 ms frames** (`frame_len()` — 160 samples at
//! 16 kHz), NOT the 512-sample windows Silero VAD wants. So capture feeds the AEC in
//! 160-sample frames; the cleaned samples accumulate and are re-windowed to 512 for
//! VAD/STT downstream — the two cadences are decoupled by a buffer, not split 512→160
//! (512 is not a multiple of 160).
//!
//! Compiled only under the `voice-io` feature.

use crate::voice::types::VoiceError;
use webrtc_audio_processing::Processor;

/// Echo canceller over one capture stream and its known render (playback) reference.
pub struct EchoCanceller {
    proc: Processor,
}

impl EchoCanceller {
    /// Construct an AEC3 processor for 16 kHz mono. Pure DSP — no audio device.
    pub fn new() -> Result<Self, VoiceError> {
        let proc =
            Processor::new(16_000).map_err(|e| VoiceError::AecFailed(format!("init: {e:?}")))?;
        Ok(Self { proc })
    }

    /// The APM frame length in samples (10 ms @ 16 kHz = 160). Render and capture
    /// frames passed to [`process`](Self::process) MUST be exactly this many samples.
    pub fn frame_len(&self) -> usize {
        self.proc.num_samples_per_frame()
    }

    /// One 10 ms tick: register the playback `render` reference, then echo-cancel the
    /// mic `capture` frame **in place**. Both slices MUST be [`frame_len`] samples.
    ///
    /// The APM is multi-channel (a frame is an iterator of channels); mono is a
    /// single-channel frame, so each slice is wrapped as a one-element iterator.
    pub fn process(&self, render: &mut [f32], capture: &mut [f32]) -> Result<(), VoiceError> {
        self.proc
            .process_render_frame([render].into_iter())
            .map_err(|e| VoiceError::AecFailed(format!("render: {e:?}")))?;
        self.proc
            .process_capture_frame([capture].into_iter())
            .map_err(|e| VoiceError::AecFailed(format!("capture: {e:?}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_len_is_10ms_at_16khz() {
        let aec = EchoCanceller::new().expect("AEC3 must construct on this platform");
        // 16 kHz × 10 ms = 160 samples — the APM's fixed frame (research R5).
        assert_eq!(aec.frame_len(), 160);
    }

    #[test]
    fn processes_a_frame_in_place_without_error() {
        // Real DSP, no audio device: feed a render reference + a capture frame and
        // confirm the AEC3 pipeline runs end-to-end on this platform.
        let aec = EchoCanceller::new().unwrap();
        let n = aec.frame_len();
        let mut render = vec![0.25_f32; n];
        let mut capture = vec![0.25_f32; n];
        aec.process(&mut render, &mut capture).expect("AEC must process a frame");
        // Output is finite (the canceller didn't diverge to NaN/inf on a steady tone).
        assert!(capture.iter().all(|s| s.is_finite()));
    }
}
