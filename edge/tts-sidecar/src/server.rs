//! Tiny HTTP server loop — single-threaded, blocking.
//!
//! Accepts POST /v1/audio/speech with JSON body:
//!   `{ "model": "…", "input": "…", "voice": "…", "speed": 1.0 }`
//!
//! Returns 200 audio/wav on success, 500 application/json on error.

use std::io::Read as _;

use misaki_rs::{Language, G2P};
use serde::Deserialize;
use tiny_http::{Method, Response, Server};

use crate::{
    kokoro::{synthesize, text_to_token_ids},
    voices::Voices,
    vocab::build_vocab,
    wav::encode_wav,
};

/// Maximum allowed request body size (64 KiB).
const MAX_BODY_BYTES: u64 = 64 * 1024;

#[derive(Deserialize)]
struct SpeechRequest {
    #[allow(dead_code)]
    model: String,
    input: String,
    voice: String,
    #[serde(default = "default_speed")]
    speed: f32,
}

fn default_speed() -> f32 {
    1.0
}

/// All shared state behind a Mutex for the session.
pub struct AppState {
    pub session: std::sync::Mutex<ort::session::Session>,
    pub voices: Voices,
    pub vocab: std::collections::HashMap<char, i64>,
    pub g2p: G2P,
}

impl AppState {
    pub fn new(model_path: &str, voices_path: &str) -> Result<Self, String> {
        eprintln!("[tts] loading ONNX model from {model_path}");
        let session = crate::kokoro::load_session(model_path)?;
        eprintln!("[tts] model loaded");

        eprintln!("[tts] loading voices from {voices_path}");
        let voices = Voices::load(voices_path)?;
        eprintln!("[tts] voices loaded ({} voices)", voices.data.len());

        let vocab = build_vocab();
        eprintln!("[tts] vocab built ({} entries)", vocab.len());

        let g2p = G2P::new(Language::EnglishUS);
        eprintln!("[tts] G2P initialised");

        Ok(Self {
            session: std::sync::Mutex::new(session),
            voices,
            vocab,
            g2p,
        })
    }
}

fn handle_speech(state: &AppState, req_body: &str) -> Result<Vec<u8>, String> {
    let req: SpeechRequest =
        serde_json::from_str(req_body).map_err(|e| format!("json parse: {e}"))?;

    let speed = req.speed.clamp(0.5, 2.0);

    let token_ids = text_to_token_ids(&req.input, &state.vocab, &state.g2p)?;
    let n_tokens = token_ids.len();

    let style = state.voices.style_vector(&req.voice, n_tokens)?;
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock: {e}"))?;
    let samples = synthesize(&mut session, &token_ids, &style, speed)?;
    let wav = encode_wav(&samples)?;

    eprintln!(
        "[tts] voice={} tokens={} samples={} wav_bytes={}",
        req.voice,
        n_tokens,
        samples.len(),
        wav.len()
    );

    Ok(wav)
}

/// Start the HTTP server and block forever, serving requests.
pub fn run(state: AppState, listen_addr: &str) -> ! {
    let server = Server::http(listen_addr)
        .unwrap_or_else(|e| panic!("failed to bind {listen_addr}: {e}"));
    eprintln!("[tts] listening on http://{listen_addr}");

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_owned();

        // Liveness probe (B1): the shell's sidecar health-wait — and
        // voice-sidecars.sh — need a 200 to know the engine is up. Without this
        // route /health returned 404, so the Tauri voice-enable health-wait never
        // succeeded and voice always surfaced a generic error.
        if method == Method::Get && url == "/health" {
            let _ = request.respond(Response::from_string("ok"));
            continue;
        }

        if method != Method::Post || url != "/v1/audio/speech" {
            let _ = request.respond(Response::from_string("not found").with_status_code(404));
            continue;
        }

        // Reject oversized bodies before reading.
        if let Some(len) = request.body_length() {
            if len as u64 > MAX_BODY_BYTES {
                let _ = request
                    .respond(Response::from_string("payload too large").with_status_code(413));
                continue;
            }
        }

        let mut body = String::new();
        if request
            .as_reader()
            .take(MAX_BODY_BYTES)
            .read_to_string(&mut body)
            .is_err()
        {
            let _ = request.respond(Response::from_string("bad request").with_status_code(400));
            continue;
        }

        match handle_speech(&state, &body) {
            Ok(wav_bytes) => {
                let resp = Response::from_data(wav_bytes)
                    .with_header(
                        "Content-Type: audio/wav"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    )
                    .with_status_code(200);
                let _ = request.respond(resp);
            }
            Err(e) => {
                eprintln!("[tts] error: {e}");
                let body = serde_json::json!({"error": e}).to_string();
                let resp = Response::from_string(body)
                    .with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    )
                    .with_status_code(500);
                let _ = request.respond(resp);
            }
        }
    }

    // tiny_http's `incoming_requests()` only returns when the server is shut
    // down — which never happens in normal operation.
    panic!("server loop exited unexpectedly");
}
