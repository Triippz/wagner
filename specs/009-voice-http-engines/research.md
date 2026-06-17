# Research: Voice HTTP Engines — HttpStt / HttpTts

## External sources

### faster-whisper-server STT API

- [OpenAI Speech-to-text API guide](https://platform.openai.com/docs/guides/speech-to-text) — Defines the `POST /v1/audio/transcriptions` endpoint with `multipart/form-data`. Required fields: `file` (audio file) and `model` (string, e.g. `"whisper-1"`). Response body: `{"text": "..."}`. faster-whisper-server implements this same interface.
- [SYSTRAN/faster-whisper on GitHub](https://github.com/SYSTRAN/faster-whisper) — CTranslate2-based faster-whisper inference. The companion HTTP server project (`fedirz/faster-whisper-server`) exposes the OpenAI-compatible transcription endpoint at `POST /v1/audio/transcriptions`.
- [fedirz/faster-whisper-server](https://github.com/fedirz/faster-whisper-server) — Docker image and API docs confirming: `POST /v1/audio/transcriptions`, multipart/form-data, field names `file` and `model`, response `{"text":"..."}`. Confidence / word timestamps available via `verbose_json` format but not in default JSON mode.

### Kokoro-FastAPI TTS API

- [remsky/Kokoro-FastAPI on GitHub](https://github.com/remsky/Kokoro-FastAPI) — OpenAI-compatible TTS API. Endpoint: `POST /v1/audio/speech`. Request body JSON: `{"model":"kokoro","input":"<text>","voice":"af_bella"}`. Response: raw audio bytes (`audio/mpeg` or `audio/wav` depending on config). No streaming required for batch use.
- [Kokoro TTS API on Railway](https://railway.com/deploy/kokoro-tts-api) — Deployment guide confirming the same OpenAI-compatible `POST /v1/audio/speech` surface and default voice `af_bella`.

### reqwest multipart feature

- [reqwest crate docs — multipart](https://docs.rs/reqwest/latest/reqwest/multipart/index.html) — The `multipart` Cargo feature flag enables `reqwest::multipart::Form`. It is NOT included in reqwest's default feature set; must be explicitly listed in `features = [...]` in `Cargo.toml`. Without this flag the `multipart` module does not compile.
- [reqwest Cargo.toml features](https://github.com/seanmonstar/reqwest/blob/master/Cargo.toml) — Confirms `multipart` is an optional feature, not bundled with `json` or `rustls-tls`.

### Hand-rolled mock HTTP server for hermetic tests

- [Tokio TcpListener docs](https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html) — `TcpListener::bind("127.0.0.1:0")` yields an ephemeral port; `listener.local_addr()` retrieves it. Standard pattern for hermetic integration tests without test framework deps.

## Internal prior art

- `edge/host/tests/permission_server.rs` — Existing hermetic mock TCP server: `TcpListener::bind("127.0.0.1:0")`, `tokio::spawn`, raw read/write of HTTP responses. This is the exact pattern to replicate for `http_voice_engines.rs`.
- `edge/host/src/cli/endpoint.rs` — Production HTTP client pattern: `reqwest::Client::builder().timeout(...)`, POST JSON, map `Err(...)` to graceful error string. `HttpStt`/`HttpTts` follow this same approach.
- `edge/host/src/voice/stt.rs`, `tts.rs` — Trait definitions `Stt::transcribe` and `Tts::synthesise`. `async_trait` already in workspace.
- `specs/007-voice/plan.md` — Steps 8-11 deferred: "HttpSttEngine/HttpTtsEngine (need running sidecars)". This spec resumes exactly there.

## Open questions surfaced by research

- **Confidence value for HTTP STT**: faster-whisper default JSON response has no confidence field. Convention: emit `1.0` (same as `FakeStt::returning`) or `0.9` as "real but unverified." Chosen: `1.0` — keep the same convention as the fake for now, revisit when verbose_json is wired.
- **Audio format for STT upload**: faster-whisper-server accepts most common formats (WAV, MP3, OGG, FLAC). `AudioChunk::bytes` is format-opaque. The HTTP adapter will upload the raw bytes with filename `audio.wav`; if the sidecar cannot decode, it returns a non-200 and we surface `SttFailed`. Format negotiation is deferred.
- **TTS audio format**: Kokoro returns `audio/mpeg` by default. The `SpeechChunk::bytes` is format-opaque. Playback decoding is deferred.
