//! Hermetic integration tests for HttpStt and HttpTts.
//!
//! No real faster-whisper or Kokoro sidecars required. Each test spins a
//! one-shot mock HTTP server on an ephemeral port (port 0), runs the adapter
//! against it, and asserts the correct Result variant.
//!
//! Real-sidecar smoke tests are marked `#[ignore]` and are run via
//! `make voice-e2e` only.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wagner_edge_host::voice::{
    stt::Stt,
    tts::Tts,
    AudioChunk, HttpStt, HttpTts, RouteRequest, VoiceError, VoiceRouter,
};

// ---------------------------------------------------------------------------
// Mock server helpers
// ---------------------------------------------------------------------------

/// Spin a one-shot TCP server that accepts one connection, reads the request
/// (to drain it from the client's send buffer), writes `response`, then closes.
/// Returns the bound address so callers can point clients at it.
async fn one_shot_server(response: &'static str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        // Drain the request so the client does not get ECONNRESET mid-write.
        let mut buf = vec![0u8; 65_536];
        // Read with a timeout-ish approach: read until we see the end of
        // headers or we fill the buffer. For our tests the request is always
        // small, so a single read is enough.
        let _ = stream.read(&mut buf).await;
        stream.write_all(response.as_bytes()).await.unwrap();
        // stream drops here → connection closed
    });
    addr
}

/// A mock server for TTS that returns raw binary bytes (not valid UTF-8).
async fn one_shot_binary_server(
    header: &'static str,
    body: Vec<u8>,
) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 65_536];
        let _ = stream.read(&mut buf).await;
        stream.write_all(header.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
    });
    addr
}

// ---------------------------------------------------------------------------
// STT hermetic tests — Story P1-A
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stt_happy_path() {
    let addr = one_shot_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 22\r\n\r\n{\"text\":\"hello world\"}",
    )
    .await;
    let base_url = format!("http://{}", addr);
    let stt = HttpStt::new(&base_url);
    let result = stt.transcribe(AudioChunk::silent(64)).await.unwrap();
    assert_eq!(result.text, "hello world");
    assert_eq!(result.confidence, 1.0);
}

#[tokio::test]
async fn stt_empty_audio_guard() {
    // No server needed — the guard must fire before any HTTP call.
    let stt = HttpStt::new("http://127.0.0.1:9"); // port 9 is discard; should never connect
    let err = stt
        .transcribe(AudioChunk::new(vec![], 16_000))
        .await
        .unwrap_err();
    assert_eq!(err, VoiceError::EmptyAudio);
}

#[tokio::test]
async fn stt_dead_port_graceful() {
    // Bind, record the address, drop the listener immediately → ECONNREFUSED.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let base_url = format!("http://{}", addr);
    let stt = HttpStt::new(&base_url);
    let err = stt.transcribe(AudioChunk::silent(64)).await.unwrap_err();
    assert!(
        matches!(err, VoiceError::SttFailed(_)),
        "expected SttFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn stt_bad_json() {
    // Valid HTTP 200, but body is not JSON at all.
    let addr = one_shot_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nnope",
    )
    .await;
    let base_url = format!("http://{}", addr);
    let stt = HttpStt::new(&base_url);
    let err = stt.transcribe(AudioChunk::silent(64)).await.unwrap_err();
    assert!(
        matches!(err, VoiceError::SttFailed(_)),
        "expected SttFailed on bad JSON, got {:?}",
        err
    );
}

#[tokio::test]
async fn stt_missing_text_field() {
    // Valid JSON, but the `text` key is absent — distinct from bad JSON.
    let addr = one_shot_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"result\":\"ok\"}",
    )
    .await;
    let base_url = format!("http://{}", addr);
    let stt = HttpStt::new(&base_url);
    let err = stt.transcribe(AudioChunk::silent(64)).await.unwrap_err();
    assert!(
        matches!(err, VoiceError::SttFailed(_)),
        "expected SttFailed when text field is missing, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// TTS hermetic tests — Story P1-B
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tts_happy_path() {
    let body = vec![0xDE_u8, 0xAD, 0xBE, 0xEF];
    let addr = one_shot_binary_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n",
        body.clone(),
    )
    .await;
    let base_url = format!("http://{}", addr);
    let tts = HttpTts::new(&base_url);
    let result = tts.synthesise("greet the operator").await.unwrap();
    assert_eq!(result.bytes, body);
    assert_eq!(result.source_text, "greet the operator");
}

#[tokio::test]
async fn tts_empty_text_guard() {
    // No server needed — the guard fires before any HTTP call.
    let tts = HttpTts::new("http://127.0.0.1:9"); // discard port
    let err = tts.synthesise("").await.unwrap_err();
    assert_eq!(err, VoiceError::EmptyTranscript);
}

#[tokio::test]
async fn tts_dead_port_graceful() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let base_url = format!("http://{}", addr);
    let tts = HttpTts::new(&base_url);
    let err = tts.synthesise("hello").await.unwrap_err();
    assert!(
        matches!(err, VoiceError::TtsFailed(_)),
        "expected TtsFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn tts_non200_graceful() {
    let addr = one_shot_server(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n",
    )
    .await;
    let base_url = format!("http://{}", addr);
    let tts = HttpTts::new(&base_url);
    let err = tts.synthesise("hello").await.unwrap_err();
    assert!(
        matches!(err, VoiceError::TtsFailed(_)),
        "expected TtsFailed on non-200, got {:?}",
        err
    );
}

#[tokio::test]
async fn tts_empty_body_graceful() {
    // HTTP 200 but zero audio bytes — not a usable response.
    let addr =
        one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
    let base_url = format!("http://{}", addr);
    let tts = HttpTts::new(&base_url);
    let err = tts.synthesise("hello").await.unwrap_err();
    assert!(
        matches!(err, VoiceError::TtsFailed(_)),
        "expected TtsFailed on empty body, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Router — Story P1-C
// ---------------------------------------------------------------------------

#[tokio::test]
async fn router_default_http_compiles_and_routes_local() {
    let router = VoiceRouter::default_http("http://127.0.0.1:8771", "http://127.0.0.1:8772");
    // Must route "local" successfully (no sidecars needed — just structural).
    let handles = router.route(&RouteRequest::new("local")).unwrap();
    let _ = handles.stt;
    let _ = handles.tts;
}

#[tokio::test]
async fn router_default_http_does_not_route_unknown_tag() {
    let router = VoiceRouter::default_http("http://127.0.0.1:8771", "http://127.0.0.1:8772");
    let err = router
        .route(&RouteRequest::new("cloud"))
        .unwrap_err();
    assert!(matches!(err, VoiceError::NoEngineMatch(_)));
}

// ---------------------------------------------------------------------------
// Real-sidecar smoke tests — marked #[ignore]; run via `make voice-e2e`
// ---------------------------------------------------------------------------

/// Requires faster-whisper-server running on 127.0.0.1:8771.
#[tokio::test]
#[ignore]
async fn stt_real_sidecar() {
    let stt = HttpStt::new("http://127.0.0.1:8771");
    let result = stt.transcribe(AudioChunk::silent(1024)).await;
    // Either Ok (sidecar transcribed silence) or a typed error — never a panic.
    match result {
        Ok(t) => println!("stt_real_sidecar: transcript = {:?}", t),
        Err(e) => println!("stt_real_sidecar: expected error = {:?}", e),
    }
}

/// Requires Kokoro-FastAPI running on 127.0.0.1:8772.
#[tokio::test]
#[ignore]
async fn tts_real_sidecar() {
    let tts = HttpTts::new("http://127.0.0.1:8772");
    let result = tts.synthesise("hello from the voice pillar").await;
    match result {
        Ok(chunk) => println!("tts_real_sidecar: {} bytes", chunk.bytes.len()),
        Err(e) => println!("tts_real_sidecar: expected error = {:?}", e),
    }
}
