# Plan: Voice HTTP Engines — HttpStt / HttpTts

**Status:** reviewed
**Branch:** feat/autonomous-build
**Spec:** ./spec.md
**Created:** 2026-06-17

## Constitution check

Wagner's constitution lives at `docs/spec/constitution.md`. Articles evaluated:

- Article I: Test-First → pass (RED test before each GREEN impl task)
- Article II: Evidence-Driven → pass (research.md cites real URLs; no training-data assertions)
- Article III: Hard-Fail CRITICAL → pass (graceful error variants, not panics; `make verify` gate enforced)
- Article IV: Independent Stories → pass (each story P1-A/B/C is independently testable without later stories)
- Article V: Simplicity Gate → pass (no new frameworks, no new heavy deps — only enabling an existing reqwest feature flag)
- Article VI: Edge-Autonomy → pass (HTTP calls are local-only; sidecars run on operator's machine; no cloud dependency added)
- Article VII: One-Directional Dependency → pass (`voice/http_stt.rs` and `voice/http_tts.rs` depend on `voice/types.rs`; no reverse deps introduced)
- Article VIII: Event-Sourced Truth → n/a (no event store touched)
- Article IX: Privacy Boundary → pass (audio bytes and text stay local; sent only to `127.0.0.1` sidecars)
- Article X: Schema-Validated Payloads → pass (STT response parsed with serde_json against known shape; TTS response is raw bytes; both error on parse failure)

All articles pass.

## Technical context

- **Language / toolchain:** Rust 2021 edition (workspace), tokio `full`, async-trait
- **Crates touched:** `wagner-edge-host` (only)
- **New files:** `edge/host/src/voice/http_stt.rs`, `edge/host/src/voice/http_tts.rs`, `edge/host/tests/http_voice_engines.rs`
- **Modified files:** `edge/host/src/voice/mod.rs` (re-exports), `edge/host/Cargo.toml` (new `[[test]]`), `Cargo.toml` (workspace, add `multipart` to reqwest features), `Makefile` (add `voice-e2e` target)
- **Transports:** HTTP/1.1 over loopback only (127.0.0.1). No wire-size budget needed.
- **Storage:** none
- **Performance goals:** STT timeout 30 s (audio processing); TTS timeout 15 s (synthesis). Graceful degradation on timeout.
- **Scale:** single operator machine; no concurrency pressure.
- **Constraints:** no new heavy dependencies; reqwest already present; `multipart` is just a feature flag addition.

## Approach

Wagner already has a trait seam (`Stt`, `Tts`) behind which the production engine adapters will live. The fakes (`FakeStt`, `FakeTts`) remain unchanged — all existing tests continue to pass against them. This plan adds two new structs (`HttpStt`, `HttpTts`) that implement the traits by calling HTTP sidecars.

`HttpStt` calls `POST /v1/audio/transcriptions` with `multipart/form-data` (OpenAI-compatible). The `reqwest` `multipart` feature flag (not yet enabled in the workspace) must be added. The response JSON `{"text":"..."}` is parsed with `serde_json`; missing `text` field or non-2xx status maps to `VoiceError::SttFailed(...)`.

`HttpTts` calls `POST /v1/audio/speech` with a JSON body `{"model":"kokoro","input":"<text>","voice":"af_bella"}`. The raw response bytes become `SpeechChunk::bytes`. Non-2xx or network errors map to `VoiceError::TtsFailed(...)`.

Both structs follow the HTTP client pattern already established in `edge/host/src/cli/endpoint.rs`: a shared `reqwest::Client` stored in the struct, constructed once with a timeout.

Hermetic tests use the same hand-rolled `tokio::net::TcpListener` pattern as `edge/host/tests/permission_server.rs`. A one-shot mock server binds on port 0 (ephemeral), returns a canned HTTP response, then exits. No framework is added. Graceful-error tests simply point at a dead port (no server running) and assert the `Err(...)` variant.

The `VoiceRouter::default_http(stt_url, tts_url)` convenience constructor wires both adapters as the `"local"` engine registration. This is a ~5-line addition to `router.rs` — no abstraction layer needed.

## Project structure

```
edge/host/src/voice/
+ http_stt.rs          # HttpStt implementing Stt trait
+ http_tts.rs          # HttpTts implementing Tts trait
~ mod.rs               # add `mod http_stt; mod http_tts;` + re-exports
~ router.rs            # add VoiceRouter::default_http() constructor

edge/host/tests/
+ http_voice_engines.rs  # hermetic mock + graceful-error tests

~ edge/host/Cargo.toml     # add [[test]] entry for http_voice_engines
~ Cargo.toml               # workspace: add `multipart` to reqwest features
~ Makefile                 # add voice-e2e target + .PHONY entry
```

## Out of scope

- Real sidecar lifecycle management (start/stop faster-whisper-server or Kokoro from Rust)
- Mic capture or audio playback
- Tauri IPC commands for voice
- Streaming responses
- Wake-word detection
- Changes to `FakeStt` / `FakeTts`
- Audio format negotiation or transcoding

## Risks & open questions

- **reqwest `multipart` feature in workspace**: adding it is a trivial feature flag; it cannot break existing code. Risk: negligible.
- **STT audio bytes format**: `AudioChunk::bytes` is format-opaque; the mock test will pass `[0u8; 64]`. The real sidecar may reject non-WAV bytes. Mitigation: sidecar returns non-200 → `SttFailed` is handled; operator ensures correct audio format via mic capture (future story).
- **Confidence value from STT**: faster-whisper default JSON has no confidence field. Using `1.0` by convention; this can be revised when `verbose_json` is wired.
- **`VoiceRouter::default_http` naming**: simple and self-explanatory. If a second production engine set is ever needed, a factory method can be added then.

## Plan Review Summary

Four critics ran (Acceptance Test, Design & Architecture, Strategic, UX skipped — no UI surface).

**Acceptance Test Critic: needs-revision → resolved**

Blockers fixed in tasks.md:
1. TDD sequence violation P1-A: `stt_bad_json` and `stt_missing_text_field` RED tests moved before the HttpStt GREEN task (bd-TBD-5a, bd-TBD-5b added before bd-TBD-6 GREEN).
2. TDD sequence violation P1-B: `tts_non200_graceful` and `tts_empty_body_graceful` RED tests moved before the HttpTts GREEN task (bd-TBD-11b, bd-TBD-11c added before bd-TBD-12 GREEN).
3. Missing edge cases: added `stt_missing_text_field` (valid JSON, missing `text` key), `tts_empty_body_graceful` (HTTP 200, zero bytes). Non-http URL guard documented as out-of-scope for this iteration (caller is production code with a fixed URL; added to risks/open questions).
4. Dead-port test reliability on macOS: documented in task description — bind, get addr, drop, immediately connect; ECONNREFUSED is returned before TIME_WAIT applies.

Remaining warnings (non-blocking, accepted):
- P1-C router test: one positive + no negative assertion is sufficient for a compile-and-wire-check; the "cloud" tag negative test is a nice-to-have, not a blocker.
- Timeout values not covered by tests: documented as known uncovered path. Hardware-level assertion on reqwest Client builder is not idiomatic in this codebase.
- `#[ignore]` smoke tests accept Ok(_) or Err(_) — intentional; they are documentation-level probes only.

**Design & Architecture Critic: approve**

Warnings accepted:
- Two `reqwest::Client` instances: justified by different timeouts (30s STT, 15s TTS). Document in constructor comments.
- `confidence: 1.0` for HTTP STT: accepted as known convention; `Transcript::new(text, 1.0)` is explicit. An `Option<f32>` refactor is deferred.
- SSRF guard: HttpStt/HttpTts only accept `127.0.0.1` by convention (operator config); no public API. Out-of-scope for this iteration.

**Strategic Critic: approve**

Warnings accepted: confidence sentinel and format opaqueness noted in research.md open questions.
