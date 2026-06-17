//! HTTP adapter for Kokoro-FastAPI — implements the `Tts` trait by calling the
//! OpenAI-compatible `POST /v1/audio/speech` endpoint.
//!
//! The adapter is deliberately thin: it translates between the Wagner domain
//! types (`SpeechChunk`, `VoiceError`) and the HTTP wire format. All engine
//! lifecycle is out of scope; the operator runs the Kokoro sidecar independently.
//!
//! # Wire format (Kokoro-FastAPI)
//!
//! Request: `POST {base_url}/v1/audio/speech`
//! Content-Type: `application/json`
//! Body: `{"model":"kokoro","input":"<text>","voice":"af_bella"}`
//!
//! Response (HTTP 200): raw audio bytes (`audio/mpeg` or similar)
//!
//! Non-2xx, empty body, or network failure → `VoiceError::TtsFailed(...)`.

use crate::voice::{
    tts::Tts,
    types::{SpeechChunk, VoiceError},
};
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

/// TTS timeout — synthesis is typically faster than STT; 15 s is generous.
/// Separate client from HttpStt (different timeout).
const TTS_TIMEOUT_SECS: u64 = 15;

/// HTTP TTS adapter for Kokoro-FastAPI (OpenAI-compatible API).
///
/// Holds a single `reqwest::Client` (internally `Arc`-wrapped connection pool)
/// created at construction time. Each `synthesise` call sends one POST request.
///
/// # Example
///
/// ```no_run
/// use wagner_edge_host::voice::HttpTts;
/// use wagner_edge_host::voice::tts::Tts;
///
/// # #[tokio::main]
/// # async fn main() {
/// let tts = HttpTts::new("http://127.0.0.1:8772");
/// let result = tts.synthesise("hello from Wagner").await;
/// # }
/// ```
pub struct HttpTts {
    client: reqwest::Client,
    base_url: String,
}

impl HttpTts {
    /// Construct an adapter pointing at `base_url` (e.g. `"http://127.0.0.1:8772"`).
    /// Trailing slashes are trimmed. The `reqwest::Client` is created once here
    /// (separate from HttpStt because TTS and STT have different timeouts).
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TTS_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self { client, base_url }
    }
}

#[async_trait]
impl Tts for HttpTts {
    /// Synthesise `text` via the Kokoro-FastAPI HTTP endpoint.
    ///
    /// Returns `Err(VoiceError::EmptyTranscript)` immediately (no HTTP call)
    /// if `text` is empty. Maps all network, non-2xx, and empty-body errors
    /// to `VoiceError::TtsFailed(...)`.
    async fn synthesise(&self, text: &str) -> Result<SpeechChunk, VoiceError> {
        if text.is_empty() {
            return Err(VoiceError::EmptyTranscript);
        }

        let url = format!("{}/v1/audio/speech", self.base_url);

        let body = json!({
            "model": "kokoro",
            "input": text,
            "voice": "af_bella"
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VoiceError::TtsFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(VoiceError::TtsFailed(format!(
                "HTTP {}: TTS sidecar returned non-2xx",
                status
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| VoiceError::TtsFailed(e.to_string()))?;

        if bytes.is_empty() {
            return Err(VoiceError::TtsFailed(
                "empty audio response from TTS sidecar".to_string(),
            ));
        }

        Ok(SpeechChunk::new(bytes.to_vec(), text))
    }
}
