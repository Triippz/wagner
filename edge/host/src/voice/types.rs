//! Shared types for the Voice pillar — speech-to-text and text-to-speech.
//!
//! All types are `Clone + Debug + PartialEq` so tests can make assertions
//! without extra ceremony.

use thiserror::Error;

/// A chunk of audio bytes captured from a microphone or a test fixture.
///
/// Wraps `Vec<u8>` so the pipeline has a named type to pass between stages
/// and is not stringly-typed. The bytes are opaque to all Voice code except
/// the real STT adapter (which is only used in production, not in tests).
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    /// Raw PCM / encoded audio bytes.
    pub bytes: Vec<u8>,
    /// Sampling rate in Hz (e.g. 16_000). `0` is valid only in test fixtures
    /// where the adapter ignores it.
    pub sample_rate_hz: u32,
}

impl AudioChunk {
    /// Create a new chunk with the given bytes and sample rate.
    pub fn new(bytes: Vec<u8>, sample_rate_hz: u32) -> Self {
        Self {
            bytes,
            sample_rate_hz,
        }
    }

    /// Convenience constructor for tests — zero-filled buffer.
    pub fn silent(len: usize) -> Self {
        Self::new(vec![0u8; len], 16_000)
    }
}

/// A transcript produced by a speech-to-text pass.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    /// The recognized text. May be empty if the audio was silent or inaudible.
    pub text: String,
    /// Confidence in `[0.0, 1.0]`. Real adapters populate this; fakes use
    /// `1.0` by convention.
    pub confidence: f32,
}

impl Transcript {
    pub fn new(text: impl Into<String>, confidence: f32) -> Self {
        Self {
            text: text.into(),
            confidence,
        }
    }

    /// High-confidence transcript for use in test fakes.
    pub fn certain(text: impl Into<String>) -> Self {
        Self::new(text, 1.0)
    }
}

/// An audio response produced by a text-to-speech pass.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechChunk {
    /// Synthesised audio bytes (PCM, mp3, opus, etc. — format is adapter-specific).
    pub bytes: Vec<u8>,
    /// The text that was synthesised (echoed back for tracing/logging).
    pub source_text: String,
}

impl SpeechChunk {
    pub fn new(bytes: Vec<u8>, source_text: impl Into<String>) -> Self {
        Self {
            bytes,
            source_text: source_text.into(),
        }
    }
}

/// Errors the Voice pipeline can produce.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum VoiceError {
    /// The audio input was empty (no bytes).
    #[error("audio chunk is empty")]
    EmptyAudio,

    /// The transcript text was empty — nothing to synthesize.
    #[error("transcript text is empty")]
    EmptyTranscript,

    /// The STT adapter could not process the audio.
    #[error("speech-to-text failed: {0}")]
    SttFailed(String),

    /// The TTS adapter could not synthesise the text.
    #[error("text-to-speech failed: {0}")]
    TtsFailed(String),

    /// The router could not select an engine for the given request.
    #[error("no engine matched for request: {0}")]
    NoEngineMatch(String),

    /// The acoustic echo canceller failed to initialise or process a frame (015).
    #[error("acoustic echo cancellation failed: {0}")]
    AecFailed(String),

    /// Microphone capture was denied (no OS permission / device unavailable) (015, FR-014).
    #[error("microphone access denied")]
    MicDenied,
}
