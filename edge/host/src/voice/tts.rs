//! Text-to-speech port — the `Tts` trait and a scripted fake for tests.
//!
//! Real adapters (local synthesiser, cloud TTS) implement `Tts` without
//! touching test code. `FakeTts` is always compiled (not cfg(test)-gated)
//! because the integration-test binary also references it.

use crate::voice::types::{SpeechChunk, VoiceError};
use async_trait::async_trait;

/// Port: convert text to audio.
#[async_trait]
pub trait Tts: Send + Sync {
    /// Synthesise the given text into audio bytes.  Never panics; errors
    /// propagate as `VoiceError`.
    async fn synthesise(&self, text: &str) -> Result<SpeechChunk, VoiceError>;
}

// ---------------------------------------------------------------------------
// Test double
// ---------------------------------------------------------------------------

/// A scripted TTS fake that echoes the input text as UTF-8 bytes (so tests
/// can verify the round-trip without a real synthesiser).
///
/// Optionally configured to fail for every call.
pub struct FakeTts {
    /// If `Some`, return this error instead of synthesising.
    error: Option<VoiceError>,
}

impl FakeTts {
    /// Always succeeds — the returned `SpeechChunk` bytes are the UTF-8
    /// encoding of the input text (a useful round-trip invariant).
    pub fn succeeding() -> Self {
        Self { error: None }
    }

    /// Always fails with the given error.
    pub fn failing(err: VoiceError) -> Self {
        Self { error: Some(err) }
    }
}

#[async_trait]
impl Tts for FakeTts {
    async fn synthesise(&self, text: &str) -> Result<SpeechChunk, VoiceError> {
        if text.is_empty() {
            return Err(VoiceError::EmptyTranscript);
        }
        if let Some(ref err) = self.error {
            return Err(err.clone());
        }
        // Echo the text as bytes — a round-trip invariant tests can assert on.
        Ok(SpeechChunk::new(text.as_bytes().to_vec(), text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_tts_echoes_text_as_bytes() {
        let tts = FakeTts::succeeding();
        let chunk = tts.synthesise("greet the operator").await.unwrap();
        assert_eq!(chunk.bytes, b"greet the operator");
        assert_eq!(chunk.source_text, "greet the operator");
    }

    #[tokio::test]
    async fn fake_tts_rejects_empty_text() {
        let tts = FakeTts::succeeding();
        let err = tts.synthesise("").await.unwrap_err();
        assert_eq!(err, VoiceError::EmptyTranscript);
    }

    #[tokio::test]
    async fn fake_tts_propagates_scripted_error() {
        let tts = FakeTts::failing(VoiceError::TtsFailed("no voice model".into()));
        let err = tts.synthesise("hello").await.unwrap_err();
        assert!(matches!(err, VoiceError::TtsFailed(_)));
    }
}
