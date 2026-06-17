//! Integration test: synthesize real audio given model + voices on disk.
//!
//! Requires environment variables:
//!   WAGNER_TTS_MODEL  — path to model_quantized.onnx
//!   WAGNER_TTS_VOICES — path to voices-v1.0.bin
//!
//! Run with:
//!   WAGNER_TTS_MODEL=… WAGNER_TTS_VOICES=… cargo test -p wagner-tts-sidecar -- --ignored
//!
//! Marked `#[ignore]` so the CI gate (no model files) skips it by default.
//!
//! Note: this test exercises the binary's server end-to-end by spinning it up
//! on a random port and sending a real HTTP request. The server process is
//! spawned as a child of the test binary.

use std::io::Read as _;
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

#[test]
#[ignore = "requires WAGNER_TTS_MODEL and WAGNER_TTS_VOICES env vars pointing to real model files"]
fn synthesize_produces_nonempty_wav() {
    let model_path = std::env::var("WAGNER_TTS_MODEL")
        .expect("WAGNER_TTS_MODEL must be set to run this test");
    let voices_path = std::env::var("WAGNER_TTS_VOICES")
        .expect("WAGNER_TTS_VOICES must be set to run this test");

    // Pick a test port that won't collide with the default 8772.
    let port = 18772u16;
    let addr = format!("127.0.0.1:{port}");

    // Spawn the sidecar binary (built by cargo before tests run).
    let bin_path = env!("CARGO_BIN_EXE_wagner-tts-sidecar");
    let mut child = Command::new(bin_path)
        .env("WAGNER_TTS_MODEL", &model_path)
        .env("WAGNER_TTS_VOICES", &voices_path)
        .env("WAGNER_TTS_PORT", port.to_string())
        .spawn()
        .expect("spawn tts sidecar");

    // Wait for the server to be ready (up to 10 s).
    let ready = (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(200));
        TcpStream::connect(&addr).is_ok()
    });
    assert!(ready, "tts sidecar did not become ready within 10 s");

    // Send a minimal POST /v1/audio/speech request.
    let body = r#"{"model":"kokoro","input":"Hello world.","voice":"af_heart"}"#;
    let request = format!(
        "POST /v1/audio/speech HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let mut stream = TcpStream::connect(&addr).expect("connect to sidecar");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    use std::io::Write as _;
    stream.write_all(request.as_bytes()).expect("send request");

    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");

    // Kill the child process and reap it to avoid zombies.
    let _ = child.kill();
    let _ = child.wait();

    // The response is raw HTTP/1.1 — find the body after \r\n\r\n.
    let header_end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("HTTP response has no header/body separator") + 4;
    let wav_body = &response[header_end..];

    assert!(
        wav_body.len() > 44,
        "expected WAV body larger than just a header, got {} bytes",
        wav_body.len()
    );
    assert_eq!(
        &wav_body[0..4],
        b"RIFF",
        "expected RIFF magic at start of WAV response"
    );
}
