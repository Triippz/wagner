//! Speech-to-text port — the `Stt` trait and a scripted fake for tests.
//!
//! Real adapters (Whisper, cloud ASR) implement `Stt` without touching any
//! test code. The `FakeStt` lives here (behind `#[cfg(test)]` is *not* done
//! because the pipeline test also needs it — instead it is always compiled but
//! clearly labelled as a test double).

use crate::voice::types::{AudioChunk, Transcript, VoiceError};
use async_trait::async_trait;

/// Port: convert audio to text.
#[async_trait]
pub trait Stt: Send + Sync {
    /// Transcribe the given audio chunk.  Never panics; errors propagate as
    /// `VoiceError`.
    async fn transcribe(&self, audio: AudioChunk) -> Result<Transcript, VoiceError>;
}

// ---------------------------------------------------------------------------
// Test double
// ---------------------------------------------------------------------------

/// A scripted STT fake that returns a fixed transcript (or a fixed error) for
/// every call regardless of the audio content.
///
/// Used in unit and integration tests that need a deterministic STT without
/// touching any real audio pipeline.
pub struct FakeStt {
    /// The result to return for every `transcribe` call.
    result: Result<Transcript, VoiceError>,
}

impl FakeStt {
    /// Always succeeds with the given text at full confidence.
    pub fn returning(text: impl Into<String>) -> Self {
        Self {
            result: Ok(Transcript::certain(text)),
        }
    }

    /// Always fails with the given error.
    pub fn failing(err: VoiceError) -> Self {
        Self { result: Err(err) }
    }
}

#[async_trait]
impl Stt for FakeStt {
    async fn transcribe(&self, audio: AudioChunk) -> Result<Transcript, VoiceError> {
        if audio.bytes.is_empty() {
            return Err(VoiceError::EmptyAudio);
        }
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_stt_returns_scripted_transcript() {
        let stt = FakeStt::returning("hello world");
        let chunk = AudioChunk::silent(128);
        let t = stt.transcribe(chunk).await.unwrap();
        assert_eq!(t.text, "hello world");
        assert_eq!(t.confidence, 1.0);
    }

    #[tokio::test]
    async fn fake_stt_rejects_empty_audio() {
        let stt = FakeStt::returning("anything");
        let chunk = AudioChunk::new(vec![], 16_000);
        let err = stt.transcribe(chunk).await.unwrap_err();
        assert_eq!(err, VoiceError::EmptyAudio);
    }

    #[tokio::test]
    async fn fake_stt_propagates_scripted_error() {
        let stt = FakeStt::failing(VoiceError::SttFailed("backend down".into()));
        let chunk = AudioChunk::silent(64);
        let err = stt.transcribe(chunk).await.unwrap_err();
        assert!(matches!(err, VoiceError::SttFailed(_)));
    }
}
