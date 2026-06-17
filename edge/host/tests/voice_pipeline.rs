//! Integration tests for the Voice pipeline — spec 007.
//!
//! These tests exercise the full `VoiceRouter → VoicePipeline` path using
//! `FakeStt` / `FakeTts` doubles.  No real audio, no network, no Python.

use std::sync::Arc;
use wagner_edge_host::voice::{
    pipeline::VoicePipeline,
    router::{RouteRequest, VoiceRouter},
    stt::FakeStt,
    tts::FakeTts,
    types::{AudioChunk, VoiceError},
};

// ---------------------------------------------------------------------------
// Router tests
// ---------------------------------------------------------------------------

#[test]
fn router_selects_registered_engine_by_tag() {
    let stt = Arc::new(FakeStt::returning("hello from fake"));
    let tts = Arc::new(FakeTts::succeeding());
    let router = VoiceRouter::new().register("local", stt, tts);
    router.route(&RouteRequest::new("local")).unwrap();
}

#[test]
fn router_rejects_unregistered_tag() {
    let router = VoiceRouter::new();
    let err = router
        .route(&RouteRequest::new("missing"))
        .unwrap_err();
    assert!(
        matches!(err, VoiceError::NoEngineMatch(_)),
        "expected NoEngineMatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Pipeline tests via router
// ---------------------------------------------------------------------------

fn build_pipeline(utterance: &str) -> VoicePipeline {
    let stt = Arc::new(FakeStt::returning(utterance));
    let tts = Arc::new(FakeTts::succeeding());
    VoicePipeline::new(stt, tts)
}

#[tokio::test]
async fn full_round_trip_produces_matching_speech() {
    let pipeline = build_pipeline("deploy the edge host");
    let result = pipeline
        .run(AudioChunk::silent(256))
        .await
        .expect("pipeline must succeed");

    assert_eq!(
        result.transcript.text, "deploy the edge host",
        "transcript must match the scripted STT output"
    );
    // FakeTts echoes text as UTF-8 bytes — the round-trip invariant.
    assert_eq!(
        result.speech.bytes,
        b"deploy the edge host",
        "speech bytes must be the UTF-8 encoding of the transcript"
    );
    assert_eq!(result.speech.source_text, "deploy the edge host");
}

#[tokio::test]
async fn pipeline_rejects_empty_audio_before_calling_stt() {
    let pipeline = build_pipeline("should never appear");
    let err = pipeline
        .run(AudioChunk::new(vec![], 16_000))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        VoiceError::EmptyAudio,
        "empty audio must be rejected before STT is called"
    );
}

#[tokio::test]
async fn pipeline_surfaces_empty_transcript_error() {
    // STT returns "" → pipeline must reject before calling TTS.
    let pipeline = VoicePipeline::new(
        Arc::new(FakeStt::returning("")),
        Arc::new(FakeTts::succeeding()),
    );
    let err = pipeline
        .run(AudioChunk::silent(64))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        VoiceError::EmptyTranscript,
        "empty transcript must be rejected before TTS is called"
    );
}

#[tokio::test]
async fn pipeline_surfaces_stt_failure() {
    let pipeline = VoicePipeline::new(
        Arc::new(FakeStt::failing(VoiceError::SttFailed(
            "microphone disconnected".into(),
        ))),
        Arc::new(FakeTts::succeeding()),
    );
    let err = pipeline.run(AudioChunk::silent(32)).await.unwrap_err();
    assert!(
        matches!(err, VoiceError::SttFailed(_)),
        "STT error must propagate"
    );
}

#[tokio::test]
async fn pipeline_surfaces_tts_failure() {
    let pipeline = VoicePipeline::new(
        Arc::new(FakeStt::returning("say something")),
        Arc::new(FakeTts::failing(VoiceError::TtsFailed(
            "no voice model loaded".into(),
        ))),
    );
    let err = pipeline.run(AudioChunk::silent(32)).await.unwrap_err();
    assert!(
        matches!(err, VoiceError::TtsFailed(_)),
        "TTS error must propagate"
    );
}

#[tokio::test]
async fn router_to_pipeline_end_to_end() {
    // Wire router → pipeline → result in one shot.
    let stt: Arc<dyn wagner_edge_host::voice::stt::Stt> =
        Arc::new(FakeStt::returning("trigger the workflow"));
    let tts: Arc<dyn wagner_edge_host::voice::tts::Tts> = Arc::new(FakeTts::succeeding());

    let router = VoiceRouter::new().register("fake", Arc::clone(&stt), Arc::clone(&tts));
    let handles = router.route(&RouteRequest::new("fake")).unwrap();
    let pipeline = VoicePipeline::new(handles.stt, handles.tts);

    let result = pipeline
        .run(AudioChunk::silent(128))
        .await
        .expect("end-to-end must succeed");

    assert_eq!(result.transcript.text, "trigger the workflow");
}

#[tokio::test]
async fn confidence_is_one_for_fake_stt() {
    let pipeline = build_pipeline("check confidence");
    let result = pipeline.run(AudioChunk::silent(16)).await.unwrap();
    assert_eq!(
        result.transcript.confidence, 1.0,
        "FakeStt must report full confidence"
    );
}
