# ADR Pointers: Voice HTTP Engines

ADRs this plan depends on or extends.

## Upstream decisions (from specs/007-voice)

- **HTTP sidecar transport (implicit ADR in specs/007-voice/plan.md §8-11)**: The voice architecture chose HTTP (not gRPC, not in-process FFI) as the integration boundary for STT and TTS engines. Reason: both faster-whisper-server and Kokoro-FastAPI expose HTTP; the existing `reqwest` client is already in the workspace; no new transport protocol needed.

- **OpenAI-compatible API surface**: Both sidecars implement the OpenAI audio API subset (`POST /v1/audio/transcriptions`, `POST /v1/audio/speech`). This was chosen for ecosystem compatibility — many local inference servers (whisper.cpp, piper, etc.) expose the same surface.

- **`reqwest` + `rustls-tls`**: Chosen in the workspace for all outbound HTTP. `HttpStt`/`HttpTts` inherit this; no alternative HTTP client considered.

## New decisions in this plan

- **`multipart` feature flag addition to `reqwest`**: STT requires `multipart/form-data` upload (per OpenAI spec). The `multipart` feature is already in the `reqwest` crate; enabling it adds no new transitive dependencies. Alternative (manual multipart body construction) was rejected as unnecessary complexity (Simplicity Gate, Article V).

- **Timeout values**: STT=30 s, TTS=15 s. Rationale: audio transcription may take multiple seconds for long clips on a CPU sidecar; TTS synthesis is typically faster. Configurable via constructor in the future if needed.

- **`VoiceRouter::default_http` convenience constructor**: A one-method addition to `router.rs` that wires `HttpStt`+`HttpTts` under the `"local"` tag. Avoids callers writing boilerplate `Arc::new(...)` pairs. The method is ~3 lines; no separate factory type introduced.

- **Confidence value `1.0` for HTTP STT**: faster-whisper JSON response has no confidence field in default mode. Using `1.0` (same as `FakeStt::returning`) for consistency. Revisit when `verbose_json` format is enabled.
