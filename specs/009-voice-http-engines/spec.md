# Spec: Voice HTTP Engines — HttpStt / HttpTts

**Status:** reviewed
**Created:** 2026-06-17
**Spec dir:** specs/009-voice-http-engines/
**Tracking:** bd-TBD-1, bd-TBD-2

## Problem

Wagner's Voice pillar already has a clean `Stt`/`Tts` trait seam and a fully
tested fake-backed pipeline (`specs/007-voice`). However, the production
adapters that connect those traits to real speech engines were explicitly
deferred from v1 scope. The `FakeStt`/`FakeTts` doubles return canned bytes;
they do not transcribe or synthesise anything. The operator therefore cannot
use voice input or spoken feedback in any real session.

The two local speech engines chosen in `specs/007-voice`'s architecture
decision — `faster-whisper-server` (STT) and Kokoro-FastAPI (TTS) — both
expose OpenAI-compatible HTTP APIs on localhost. The Rust voice module can
reach them via the already-present `reqwest` dependency, following exactly the
pattern established by `edge/host/src/cli/endpoint.rs`. Sidecars run as
separate processes started by the operator; the edge host must call them
gracefully when up and degrade gracefully when down.

## User stories

### Story P1-A: HttpStt transcribes audio via the faster-whisper sidecar

**Who:** Wagner's voice pipeline running on an operator's machine.
**What:** POST audio bytes to the faster-whisper-server HTTP endpoint and
receive a `Transcript` struct.
**Why:** Enables real-time operator voice input without any Python code in the
Rust process.

**Independent test:** Given a running mock HTTP server that returns
`{"text":"hello world"}`, `HttpStt::transcribe` returns
`Transcript { text: "hello world", confidence: 1.0 }` with no panic and no
external process invocation.

**Acceptance scenarios:**

- Given a mock faster-whisper-server bound on a random port returning
  `{"text":"run the tests"}`, when `HttpStt::transcribe(AudioChunk::silent(64))`
  is called, then it returns `Ok(Transcript { text: "run the tests", .. })`.
- Given an `AudioChunk` with empty bytes, when `transcribe` is called, then it
  returns `Err(VoiceError::EmptyAudio)` without making any HTTP call.
- Given the sidecar is not running (dead port), when `transcribe` is called with
  a non-empty chunk, then it returns `Err(VoiceError::SttFailed(_))` (not a
  panic).
- Given the sidecar returns malformed JSON, when `transcribe` is called, then
  it returns `Err(VoiceError::SttFailed(_))` with a descriptive message.

### Story P1-B: HttpTts synthesises text via the Kokoro sidecar

**Who:** Wagner's voice pipeline running on an operator's machine.
**What:** POST text to the Kokoro-FastAPI HTTP endpoint and receive audio bytes
as a `SpeechChunk`.
**Why:** Enables spoken feedback from the engine without any Python in the Rust
process.

**Independent test:** Given a running mock HTTP server that returns raw bytes
`[1u8, 2, 3]`, `HttpTts::synthesise("hello")` returns
`SpeechChunk { bytes: vec![1,2,3], source_text: "hello" }` with no panic.

**Acceptance scenarios:**

- Given a mock Kokoro server returning `[0xDE, 0xAD, 0xBE, 0xEF]` audio bytes,
  when `HttpTts::synthesise("greet the operator")` is called, then it returns
  `Ok(SpeechChunk { bytes: vec![0xDE, 0xAD, 0xBE, 0xEF], .. })`.
- Given `text` is empty, when `synthesise("")` is called, then it returns
  `Err(VoiceError::EmptyTranscript)` without making any HTTP call.
- Given the sidecar is not running (dead port), when `synthesise("hello")` is
  called, then it returns `Err(VoiceError::TtsFailed(_))` (not a panic).
- Given the sidecar returns a non-2xx status, when `synthesise("hello")` is
  called, then it returns `Err(VoiceError::TtsFailed(_))` with the status code
  in the message.

### Story P1-C: HttpStt/HttpTts are re-exported from voice::mod and wired as the production default in VoiceRouter construction

**Who:** Application code (e.g. Tauri IPC wiring, future CLI commands) that
wants a production `VoiceRouter` without writing boilerplate.
**What:** `voice::HttpStt` and `voice::HttpTts` are publicly accessible via
`mod.rs`, and a `VoiceRouter::default_http(stt_url, tts_url)` constructor
(or equivalent factory) wires them as the `"local"` engine registration.
**Why:** Reduces the integration surface for callers; they call one function
with two URLs, get a ready router.

**Independent test:** `VoiceRouter::default_http("http://127.0.0.1:8771", "http://127.0.0.1:8772")` compiles and produces a router that can be asked for the `"local"` engine tag without error (no live sidecars needed — just structural).

**Acceptance scenarios:**

- When `use wagner_edge_host::voice::HttpStt` and `use wagner_edge_host::voice::HttpTts` are written in test code, they compile without error.
- When `VoiceRouter::default_http("http://127.0.0.1:8771", "http://127.0.0.1:8772")` is called, it returns a `VoiceRouter` with a `"local"` registration; `router.route(&RouteRequest::new("local"))` succeeds.

## Edge cases

- Sidecar returns HTTP 200 but `text` field is missing from JSON (STT) → `SttFailed`.
- Sidecar returns HTTP 200 but body is empty (TTS) → treat as `TtsFailed` (zero audio bytes are not a usable response).
- Sidecar URL has trailing slash → normalize before appending path (as `endpoint.rs` does).
- `reqwest` timeout: use 30 s for STT (audio processing may take time), 15 s for TTS.
- Non-http(s) URL passed to constructor → return error variant immediately without dialing.

## Non-goals

- Real sidecar process lifecycle management (start/stop faster-whisper-server or Kokoro from Rust) — deferred; operators start sidecars independently.
- Mic capture or audio playback — deferred.
- Tauri IPC commands for voice — deferred.
- Streaming responses from STT or TTS — deferred; batch only for now.
- Wake-word detection — deferred.
- Any changes to `FakeStt` or `FakeTts` — they must remain exactly as-is.

## Review & acceptance checklist

- [ ] Every P1 story has an Independent Test.
- [ ] Acceptance scenarios cover at least one happy path and one error path per story.
- [ ] Non-goals are explicit.
- [ ] No tech stack choices appear in this file.
- [ ] External sources cited in research.md, not here.
