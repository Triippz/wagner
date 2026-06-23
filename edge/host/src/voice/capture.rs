//! Microphone capture (spec 015, T006) — turns a held push-to-talk utterance into
//! a 16 kHz mono WAV [`AudioChunk`] the whisper STT sidecar accepts (it receives the
//! bytes as a multipart `audio.wav`, see `http_stt`).
//!
//! The device path (`cpal::Stream` open in [`MicCapture`]) is gated behind the
//! `voice-io` feature; the audio transforms below — downmix, resample, PCM16,
//! WAV-encode — are pure and tested headless, so the framing logic is verified
//! without a microphone.

use crate::voice::types::AudioChunk;

/// Whisper wants 16 kHz mono.
pub const TARGET_HZ: u32 = 16_000;

/// Average interleaved `channels`-channel f32 samples down to mono.
pub fn to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let channels = channels.max(1) as usize;
    if channels == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Linear-resample mono `input` from `from_hz` to `to_hz`. Good enough for
/// whisper-tiny speech recognition; not a brick-wall anti-alias filter.
/// ponytail: linear interp, fine for tiny.en PTT — swap for `rubato` only if a
/// downstream model proves sensitive to aliasing.
pub fn resample_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.len() < 2 {
        return input.to_vec();
    }
    let ratio = to_hz as f64 / from_hz as f64;
    let out_len = ((input.len() as f64) * ratio).round().max(1.0) as usize;
    let last = input.len() - 1;
    (0..out_len)
        .map(|i| {
            let src = i as f64 / ratio;
            let j = src.floor() as usize;
            let frac = (src - j as f64) as f32;
            let a = input[j.min(last)];
            let b = input[(j + 1).min(last)];
            a + (b - a) * frac
        })
        .collect()
}

/// Clamp f32 `[-1.0, 1.0]` samples to signed 16-bit PCM.
pub fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Wrap PCM16 mono `samples` in a canonical 44-byte-header WAV (RIFF/PCM) container.
pub fn pcm16_wav(samples: &[i16], sample_rate_hz: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = sample_rate_hz * 2; // mono, 16-bit → 2 bytes/sample
    let mut buf = Vec::with_capacity(44 + data_len as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // channels = mono
    buf.extend_from_slice(&sample_rate_hz.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align = channels * bits/8
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

/// Full PTT encode: interleaved device f32 @ `from_hz` → 16 kHz mono WAV [`AudioChunk`].
pub fn encode_utterance(interleaved: &[f32], channels: u16, from_hz: u32) -> AudioChunk {
    let mono = to_mono(interleaved, channels);
    let resampled = resample_linear(&mono, from_hz, TARGET_HZ);
    let pcm = to_pcm16(&resampled);
    AudioChunk::new(pcm16_wav(&pcm, TARGET_HZ), TARGET_HZ)
}

#[cfg(feature = "voice-io")]
pub use device::MicCapture;

// MicCapture must be Send + Sync to live in Tauri-managed state between the PTT
// start/stop IPC calls — the `!Send` cpal stream stays on its own thread.
#[cfg(feature = "voice-io")]
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MicCapture>();
};

#[cfg(feature = "voice-io")]
mod device {
    use super::{encode_utterance, AudioChunk};
    use crate::voice::types::VoiceError;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::JoinHandle;
    use std::time::Duration;

    /// A held-to-talk microphone capture. The `cpal::Stream` is `!Send` on macOS,
    /// so it lives entirely on a dedicated thread; this handle holds only `Send`
    /// state and can sit in Tauri-managed state between the start and stop IPC
    /// calls. [`start`](Self::start) opens the stream and accumulates samples;
    /// [`stop`](Self::stop) ends it and returns a 16 kHz mono WAV [`AudioChunk`].
    pub struct MicCapture {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
        buf: Arc<Mutex<Vec<f32>>>,
        channels: u16,
        sample_rate: u32,
    }

    impl MicCapture {
        /// Open the default input device on a capture thread and start
        /// accumulating an utterance. Returns [`VoiceError::MicDenied`] when no
        /// input device is available (no permission / no device).
        pub fn start() -> Result<Self, VoiceError> {
            let stop = Arc::new(AtomicBool::new(false));
            let buf = Arc::new(Mutex::new(Vec::<f32>::new()));
            // The thread reports the live stream's format (or the build error) back
            // once the stream is playing, so `start()` fails fast on a denied mic.
            let (ready_tx, ready_rx) = mpsc::channel::<Result<(u16, u32), VoiceError>>();
            let stop_t = Arc::clone(&stop);
            let buf_t = Arc::clone(&buf);

            let thread = std::thread::spawn(move || {
                let built = build_stream(&buf_t);
                match built {
                    Ok((stream, channels, sample_rate)) => {
                        // Keep the `!Send` stream owned by this thread, alive until
                        // stop is signalled — the callback fires on cpal's audio thread.
                        let _ = ready_tx.send(Ok((channels, sample_rate)));
                        while !stop_t.load(Ordering::Relaxed) {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            });

            match ready_rx.recv() {
                Ok(Ok((channels, sample_rate))) => {
                    Ok(Self { stop, thread: Some(thread), buf, channels, sample_rate })
                }
                Ok(Err(e)) => {
                    let _ = thread.join();
                    Err(e)
                }
                Err(_) => Err(VoiceError::SttFailed("capture thread exited before reporting".into())),
            }
        }

        /// Stop capture (join the thread) and return the held utterance as a
        /// 16 kHz mono WAV chunk. Returns [`VoiceError::MicDenied`] when the stream
        /// delivered only silence (a denied mic permission on macOS feeds a valid
        /// but all-zero stream) so callers surface a clear error instead of
        /// transcribing silence into a bogus goal.
        pub fn stop(mut self) -> Result<AudioChunk, VoiceError> {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            let samples = self.buf.lock().unwrap();
            // Diagnostic so a silent capture is distinguishable from an empty one:
            //   0 samples            → the stream never delivered audio (device/timing)
            //   samples, peak ≈ 0.0  → captured silence (mic permission denied / muted)
            //   samples, peak > 0    → real audio reached us
            let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            let secs = samples.len() as f32
                / (self.sample_rate.max(1) * u32::from(self.channels.max(1))) as f32;
            eprintln!(
                "[wagner] voice-capture: {} samples @ {} Hz, {} ch ({secs:.2}s), peak {peak:.4}",
                samples.len(),
                self.sample_rate,
                self.channels,
            );
            // Pure silence (exact zeros) means the OS gave us no audio — almost
            // always a denied mic permission. Real ambient noise is well above this.
            if peak < 1e-5 {
                return Err(VoiceError::MicDenied);
            }
            Ok(encode_utterance(&samples, self.channels, self.sample_rate))
        }
    }

    /// Open the default input device and build a playing stream that accumulates
    /// f32 samples into `buf`, whatever the device's native sample format.
    /// Returns the live stream plus its channel count and sample rate.
    fn build_stream(buf: &Arc<Mutex<Vec<f32>>>) -> Result<(cpal::Stream, u16, u32), VoiceError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(VoiceError::MicDenied)?;
        let config = device
            .default_input_config()
            .map_err(|e| VoiceError::SttFailed(format!("mic config: {e}")))?;
        let channels = config.channels();
        let sample_rate = config.sample_rate().0;
        let cfg = config.config();
        let sink = Arc::clone(buf);
        let err_fn = |e| eprintln!("[wagner] voice-capture: mic stream error: {e}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &cfg,
                move |data: &[f32], _: &_| sink.lock().unwrap().extend_from_slice(data),
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &cfg,
                move |data: &[i16], _: &_| {
                    let mut g = sink.lock().unwrap();
                    g.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &cfg,
                move |data: &[u16], _: &_| {
                    let mut g = sink.lock().unwrap();
                    g.extend(data.iter().map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0));
                },
                err_fn,
                None,
            ),
            other => {
                return Err(VoiceError::SttFailed(format!("unsupported sample format: {other:?}")))
            }
        }
        .map_err(|e| VoiceError::SttFailed(format!("mic stream: {e}")))?;

        stream
            .play()
            .map_err(|e| VoiceError::SttFailed(format!("mic play: {e}")))?;
        Ok((stream, channels, sample_rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_mono_averages_channels() {
        // Interleaved stereo: L,R,L,R → per-frame average.
        assert_eq!(to_mono(&[1.0, 3.0, 2.0, 4.0], 2), vec![2.0, 3.0]);
        // Mono passes through untouched.
        assert_eq!(to_mono(&[0.5, -0.5], 1), vec![0.5, -0.5]);
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let s = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&s, 16_000, 16_000), s);
    }

    #[test]
    fn resample_downsamples_48k_to_16k_by_a_third() {
        // 48k → 16k is a 1:3 ratio: ~300 samples → ~100.
        let input: Vec<f32> = (0..300).map(|i| i as f32).collect();
        let out = resample_linear(&input, 48_000, 16_000);
        assert_eq!(out.len(), 100);
        assert!((out[0] - 0.0).abs() < 1e-3, "first sample preserved");
        // Linear interp keeps it monotone-increasing for a ramp.
        assert!(out.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn pcm16_clamps_and_scales() {
        assert_eq!(to_pcm16(&[1.0, -1.0, 0.0, 2.0, -2.0]), vec![32767, -32767, 0, 32767, -32767]);
    }

    #[test]
    fn wav_header_is_canonical_pcm16() {
        let wav = pcm16_wav(&[0, 1, -1], 16_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // 44-byte header + 3 samples * 2 bytes.
        assert_eq!(wav.len(), 44 + 6);
        // RIFF chunk size = 36 + data_len(6) = 42.
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 42);
        // data chunk size = 6.
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        // sample rate field.
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
    }

    #[test]
    fn encode_utterance_yields_16k_wav_chunk() {
        // Stereo @ 48k → mono 16k WAV.
        let interleaved: Vec<f32> = (0..480).map(|i| (i % 2) as f32 * 0.5).collect();
        let chunk = encode_utterance(&interleaved, 2, 48_000);
        assert_eq!(chunk.sample_rate_hz, 16_000);
        assert_eq!(&chunk.bytes[0..4], b"RIFF");
        // 240 mono frames @48k → ~80 @16k → 44 + 80*2 bytes.
        assert_eq!(chunk.bytes.len(), 44 + 80 * 2);
    }
}
