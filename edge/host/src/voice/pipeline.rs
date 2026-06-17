//! Voice pipeline — wires STT → TTS into a single call.
//!
//! `VoicePipeline` accepts a `(Stt, Tts)` pair (usually resolved by
//! `VoiceRouter`) and drives audio through the full round-trip:
//!
//!   `AudioChunk` → `Stt::transcribe` → `Transcript` → `Tts::synthesise` → `SpeechChunk`
//!
//! The pipeline owns nothing heavy: real audio capture and playback live
//! outside this module, keeping the unit under test fast and allocation-cheap.

use crate::voice::{
    stt::Stt,
    tts::Tts,
    types::{AudioChunk, SpeechChunk, Transcript, VoiceError},
};
use std::sync::Arc;

/// The output of a complete STT → TTS round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineResult {
    /// The text the STT produced.
    pub transcript: Transcript,
    /// The audio the TTS produced.
    pub speech: SpeechChunk,
}

/// Drives audio through speech-to-text and then text-to-speech.
pub struct VoicePipeline {
    stt: Arc<dyn Stt>,
    tts: Arc<dyn Tts>,
}

impl VoicePipeline {
    /// Create a pipeline from already-resolved engine handles.
    pub fn new(stt: Arc<dyn Stt>, tts: Arc<dyn Tts>) -> Self {
        Self { stt, tts }
    }

    /// Run `audio` through the full STT → TTS pipeline.
    ///
    /// Returns `VoiceError::EmptyAudio` before calling STT if the chunk is
    /// empty, and `VoiceError::EmptyTranscript` before calling TTS if STT
    /// produced no text.  All other errors propagate from the adapters.
    pub async fn run(&self, audio: AudioChunk) -> Result<PipelineResult, VoiceError> {
        if audio.bytes.is_empty() {
            return Err(VoiceError::EmptyAudio);
        }

        let transcript = self.stt.transcribe(audio).await?;

        if transcript.text.is_empty() {
            return Err(VoiceError::EmptyTranscript);
        }

        let speech = self.tts.synthesise(&transcript.text).await?;

        Ok(PipelineResult { transcript, speech })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::stt::FakeStt;
    use crate::voice::tts::FakeTts;

    fn pipeline(text: &str) -> VoicePipeline {
        VoicePipeline::new(
            Arc::new(FakeStt::returning(text)),
            Arc::new(FakeTts::succeeding()),
        )
    }

    #[tokio::test]
    async fn pipeline_round_trips_audio_to_speech() {
        let p = pipeline("run the tests");
        let result = p.run(AudioChunk::silent(64)).await.unwrap();
        assert_eq!(result.transcript.text, "run the tests");
        // FakeTts echoes text as bytes.
        assert_eq!(result.speech.bytes, b"run the tests");
        assert_eq!(result.speech.source_text, "run the tests");
    }

    #[tokio::test]
    async fn pipeline_rejects_empty_audio() {
        let p = pipeline("anything");
        let err = p.run(AudioChunk::new(vec![], 16_000)).await.unwrap_err();
        assert_eq!(err, VoiceError::EmptyAudio);
    }

    #[tokio::test]
    async fn pipeline_rejects_empty_transcript() {
        // FakeStt returning "" → the pipeline should surface EmptyTranscript
        // before even calling TTS.
        let p = VoicePipeline::new(
            Arc::new(FakeStt::returning("")),
            Arc::new(FakeTts::succeeding()),
        );
        let err = p.run(AudioChunk::silent(64)).await.unwrap_err();
        assert_eq!(err, VoiceError::EmptyTranscript);
    }

    #[tokio::test]
    async fn pipeline_propagates_stt_error() {
        let p = VoicePipeline::new(
            Arc::new(FakeStt::failing(VoiceError::SttFailed("mic off".into()))),
            Arc::new(FakeTts::succeeding()),
        );
        let err = p.run(AudioChunk::silent(32)).await.unwrap_err();
        assert!(matches!(err, VoiceError::SttFailed(_)));
    }

    #[tokio::test]
    async fn pipeline_propagates_tts_error() {
        let p = VoicePipeline::new(
            Arc::new(FakeStt::returning("some text")),
            Arc::new(FakeTts::failing(VoiceError::TtsFailed(
                "voice model absent".into(),
            ))),
        );
        let err = p.run(AudioChunk::silent(32)).await.unwrap_err();
        assert!(matches!(err, VoiceError::TtsFailed(_)));
    }

    #[tokio::test]
    async fn pipeline_confidence_is_preserved() {
        let p = pipeline("hello");
        let result = p.run(AudioChunk::silent(16)).await.unwrap();
        // FakeStt always returns confidence 1.0.
        assert_eq!(result.transcript.confidence, 1.0);
    }
}
