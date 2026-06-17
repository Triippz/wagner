//! HTTP adapter for faster-whisper-server — implements the `Stt` trait by
//! calling the OpenAI-compatible `POST /v1/audio/transcriptions` endpoint.
//!
//! The adapter is deliberately thin: it translates between the Wagner domain
//! types (`AudioChunk`, `Transcript`, `VoiceError`) and the HTTP wire format.
//! All engine lifecycle (starting, stopping faster-whisper-server) is out of
//! scope; the operator runs the sidecar independently.
//!
//! # Wire format (faster-whisper-server)
//!
//! Request: `POST {base_url}/v1/audio/transcriptions`
//! Content-Type: `multipart/form-data`
//! Fields: `file` (audio bytes, filename `audio.wav`) and `model` (`whisper-1`)
//!
//! Response (HTTP 200): `{"text": "<transcript>"}`
//!
//! Non-2xx or parse failure → `VoiceError::SttFailed(...)`.

use crate::voice::{
    stt::Stt,
    types::{AudioChunk, Transcript, VoiceError},
};
use async_trait::async_trait;
use std::time::Duration;

/// STT timeout — audio transcription on a CPU sidecar can take several seconds
/// for longer clips. 30 s gives headroom without blocking indefinitely.
const STT_TIMEOUT_SECS: u64 = 30;

/// HTTP STT adapter for faster-whisper-server (OpenAI-compatible API).
///
/// Holds a single `reqwest::Client` (internally `Arc`-wrapped connection pool)
/// created at construction time. Each `transcribe` call sends one POST request.
///
/// # Example
///
/// ```no_run
/// use wagner_edge_host::voice::{HttpStt, AudioChunk};
/// use wagner_edge_host::voice::stt::Stt;
///
/// # #[tokio::main]
/// # async fn main() {
/// let stt = HttpStt::new("http://127.0.0.1:8771");
/// let result = stt.transcribe(AudioChunk::silent(512)).await;
/// # }
/// ```
pub struct HttpStt {
    client: reqwest::Client,
    base_url: String,
}

impl HttpStt {
    /// Construct an adapter pointing at `base_url` (e.g. `"http://127.0.0.1:8771"`).
    /// Trailing slashes are trimmed. The `reqwest::Client` is created once here
    /// (separate from HttpTts because STT and TTS have different timeouts).
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(STT_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self { client, base_url }
    }
}

#[async_trait]
impl Stt for HttpStt {
    /// Transcribe `audio` via the faster-whisper-server HTTP endpoint.
    ///
    /// Returns `Err(VoiceError::EmptyAudio)` immediately (no HTTP call) if
    /// `audio.bytes` is empty. Maps all network and parse errors to
    /// `VoiceError::SttFailed(...)`.
    async fn transcribe(&self, audio: AudioChunk) -> Result<Transcript, VoiceError> {
        if audio.bytes.is_empty() {
            return Err(VoiceError::EmptyAudio);
        }

        let url = format!("{}/v1/audio/transcriptions", self.base_url);

        // Build multipart/form-data body.
        // The `file` part carries the raw audio bytes; `model` selects the
        // faster-whisper model (OpenAI-compatible field).
        let part = reqwest::multipart::Part::bytes(audio.bytes)
            .file_name("audio.wav")
            .mime_str("application/octet-stream")
            .map_err(|e| VoiceError::SttFailed(format!("multipart error: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-1");

        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| VoiceError::SttFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(VoiceError::SttFailed(format!(
                "HTTP {}: STT sidecar returned non-2xx",
                status
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| VoiceError::SttFailed(e.to_string()))?;

        // Parse JSON response: {"text": "..."}
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| VoiceError::SttFailed(format!("JSON parse error: {e}")))?;

        let transcript_text = value
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VoiceError::SttFailed(
                    "missing 'text' field in faster-whisper-server response".to_string(),
                )
            })?;

        // faster-whisper default JSON response has no confidence field.
        // Convention: use 1.0 — same as FakeStt::returning. Revisit when
        // verbose_json format is enabled.
        Ok(Transcript::new(transcript_text, 1.0))
    }
}
