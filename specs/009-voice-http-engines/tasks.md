# Tasks: Voice HTTP Engines — HttpStt / HttpTts

**Spec:** ./spec.md
**Plan:** ./plan.md
**Status:** draft

Placeholder IDs. Run `/plan-to-jira` and `bd create` before starting execution.

## Legend

- `[ID]` — placeholder task ID
- `[P]` — parallelizable alongside other `[P]` tasks in its phase
- `[StoryRef]` — spec story this task serves
- `[ ]` — not started; `[x]` — done

---

## Phase 0 — Foundational: enable reqwest `multipart` feature

These changes unblock the STT multipart upload. Do first; nothing depends on
them being deferred.

- `[bd-TBD-1]` `[Foundational]` `[ ]` Add `"multipart"` to the `reqwest` features list in `/Users/marktripoli/Development/wagner/Cargo.toml` (workspace). Change: `features = ["json", "rustls-tls"]` → `features = ["json", "rustls-tls", "multipart"]`. Run `cargo build -p wagner-edge-host` to confirm it compiles.

---

## Phase 1 — Story P1-A: HttpStt

### Checkpoint — Independent test

After this phase, Story P1-A's Independent Test must pass:
`cargo test -p wagner-edge-host --test http_voice_engines stt`

- `[bd-TBD-2]` `[P1-A]` `[ ]` **RED**: Write failing test `stt_happy_path` in `edge/host/tests/http_voice_engines.rs`. Spin a `TcpListener` on port 0, `tokio::spawn` a one-shot handler that returns:
  ```
  HTTP/1.1 200 OK\r\nContent-Length: 22\r\n\r\n{"text":"hello world"}
  ```
  Call `HttpStt::new(base_url).transcribe(AudioChunk::silent(64)).await` and assert `Ok(Transcript { text: "hello world", confidence: 1.0 })`. Test must fail to compile or fail at runtime because `HttpStt` does not exist yet.

- `[bd-TBD-3]` `[P1-A]` `[ ]` **RED**: Add test `stt_empty_audio_guard` to the same file: call `HttpStt::new(url).transcribe(AudioChunk::new(vec![], 16_000)).await` and assert `Err(VoiceError::EmptyAudio)`.

- `[bd-TBD-4]` `[P1-A]` `[ ]` **RED**: Add test `stt_dead_port_graceful` — bind a `TcpListener` on port 0, get the address, drop the listener (do NOT accept), immediately call `transcribe` pointing at that address. On macOS TIME_WAIT is not a concern for connection-refused because the OS kernel returns ECONNREFUSED before TIME_WAIT applies. Assert `Err(VoiceError::SttFailed(_))`.

- `[bd-TBD-5a]` `[P1-A]` `[ ]` **RED** (malformed JSON): Add test `stt_bad_json` — mock server returns `HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nnope`. Assert `Err(VoiceError::SttFailed(_))`.

- `[bd-TBD-5b]` `[P1-A]` `[ ]` **RED** (missing `text` field): Add test `stt_missing_text_field` — mock server returns `HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{"result":"ok"}`. Assert `Err(VoiceError::SttFailed(_))`. This is distinct from malformed JSON — the body is valid JSON but the `text` key is absent.

- `[bd-TBD-6]` `[P1-A]` `[ ]` **GREEN**: Create `edge/host/src/voice/http_stt.rs`. Implement `HttpStt { client: reqwest::Client, base_url: String }` with `HttpStt::new(base_url: impl Into<String>) -> Self`. Implement `Stt for HttpStt`: guard empty audio → `EmptyAudio`; build multipart form with field `file` (bytes, filename `audio.wav`) and field `model` (`whisper-1`); POST to `{base_url}/v1/audio/transcriptions`; parse JSON — if `text` field absent map to `SttFailed("missing 'text' field in response")`; map network/parse errors → `SttFailed(e.to_string())`; on success return `Transcript::new(text, 1.0)`. Add `mod http_stt;` + `pub use http_stt::HttpStt;` to `edge/host/src/voice/mod.rs`. All five RED tests (stt_happy_path, stt_empty_audio_guard, stt_dead_port_graceful, stt_bad_json, stt_missing_text_field) must now pass.

- `[bd-TBD-7]` `[P1-A]` `[ ]` **REFACTOR**: Review `http_stt.rs` for clone count, error message quality, and doc comments. No behaviour change.

- `[bd-TBD-8]` `[P1-A]` `[ ]` **REFACTOR**: Review `http_stt.rs` for clone count, error message quality, and doc comments. No behaviour change.

---

## Phase 2 — Story P1-B: HttpTts

### Checkpoint — Independent test

After this phase, Story P1-B's Independent Test must pass:
`cargo test -p wagner-edge-host --test http_voice_engines tts`

- `[bd-TBD-9]` `[P1-B]` `[ ]` **RED**: Add test `tts_happy_path` — mock server returns:
  ```
  HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n\xDE\xAD\xBE\xEF
  ```
  Call `HttpTts::new(base_url).synthesise("greet the operator").await`, assert `Ok(SpeechChunk { bytes: vec![0xDE,0xAD,0xBE,0xEF], source_text: "greet the operator".to_string() })`.

- `[bd-TBD-10]` `[P1-B]` `[ ]` **RED**: Add test `tts_empty_text_guard` — call `synthesise("")` and assert `Err(VoiceError::EmptyTranscript)`.

- `[bd-TBD-11]` `[P1-B]` `[ ]` **RED**: Add test `tts_dead_port_graceful` — same bind-drop-point pattern as stt_dead_port_graceful; assert `Err(VoiceError::TtsFailed(_))`.

- `[bd-TBD-11b]` `[P1-B]` `[ ]` **RED**: Add test `tts_non200_graceful` — mock server returns `HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n`. Assert `Err(VoiceError::TtsFailed(_))`.

- `[bd-TBD-11c]` `[P1-B]` `[ ]` **RED**: Add test `tts_empty_body_graceful` — mock server returns `HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n`. Assert `Err(VoiceError::TtsFailed(_))` (zero audio bytes are not a usable response).

- `[bd-TBD-12]` `[P1-B]` `[ ]` **GREEN**: Create `edge/host/src/voice/http_tts.rs`. Implement `HttpTts { client: reqwest::Client, base_url: String }`. Implement `Tts for HttpTts`: guard empty text → `EmptyTranscript`; POST JSON `{"model":"kokoro","input":"<text>","voice":"af_bella"}` to `{base_url}/v1/audio/speech`; on non-2xx → `TtsFailed("HTTP <status>: ...")`; collect response bytes; if empty → `TtsFailed("empty audio response from TTS sidecar")`; return `SpeechChunk::new(bytes, text)`. Add `mod http_tts;` + `pub use http_tts::HttpTts;` to `mod.rs`. All six RED tests (tts_happy_path, tts_empty_text_guard, tts_dead_port_graceful, tts_non200_graceful, tts_empty_body_graceful) pass.

- `[bd-TBD-13]` `[P1-B]` `[ ]` **REFACTOR**: Review `http_tts.rs` for clone count, error message quality, and doc comments. No behaviour change.

---

## Phase 3 — Story P1-C: re-exports + VoiceRouter::default_http

### Checkpoint — Independent test

After this phase, Story P1-C's Independent Test must pass:
`cargo test -p wagner-edge-host --test http_voice_engines router`

- `[bd-TBD-16]` `[P1-C]` `[ ]` **RED**: Add test `router_default_http_compiles` — call `VoiceRouter::default_http("http://127.0.0.1:8771", "http://127.0.0.1:8772")`, then `router.route(&RouteRequest::new("local"))`, assert `Ok(_)`. This will fail because `VoiceRouter::default_http` does not exist.

- `[bd-TBD-17]` `[P1-C]` `[ ]` **GREEN**: Add `VoiceRouter::default_http(stt_url: impl Into<String>, tts_url: impl Into<String>) -> Self` to `edge/host/src/voice/router.rs`. Implementation: `VoiceRouter::new().register("local", Arc::new(HttpStt::new(stt_url)), Arc::new(HttpTts::new(tts_url)))` (or equivalent to whatever `register` signature requires). Confirm `mod.rs` already re-exports `HttpStt` and `HttpTts` (done in phases 1-2). Test passes.

- `[bd-TBD-18]` `[P1-C]` `[ ]` **REFACTOR**: no changes needed; the method is three lines.

---

## Phase 4 — Wire up test target + Makefile + `make verify`

- `[bd-TBD-19]` `[Integration]` `[ ]` Add `[[test]]` entry to `edge/host/Cargo.toml`:
  ```toml
  [[test]]
  name = "http_voice_engines"
  path = "tests/http_voice_engines.rs"
  ```

- `[bd-TBD-20]` `[Integration]` `[ ]` Add `voice-e2e` target to `Makefile` (append, no other lines changed):
  ```makefile
  voice-e2e:
  	cargo test -p wagner-edge-host --test http_voice_engines -- --ignored --nocapture
  ```
  Add `voice-e2e` to the `.PHONY` line.

- `[bd-TBD-21]` `[Integration]` `[ ]` Add real-sidecar smoke tests (all `#[ignore]`) inside `http_voice_engines.rs`:
  - `stt_real_sidecar` — calls `http://127.0.0.1:8771/v1/audio/transcriptions` with a short silent chunk. Not a correctness assertion; just verifies no panic and either `Ok(_)` or `SttFailed(_)` (sidecar may not be running).
  - `tts_real_sidecar` — calls `http://127.0.0.1:8772/v1/audio/speech`. Same.

- `[bd-TBD-22]` `[Integration]` `[ ]` Run `make verify`. Must exit 0. All hermetic tests run; `#[ignore]` tests are skipped.

---

## Phase 5 — Commit

- `[bd-TBD-23]` `[Integration]` `[ ]` Stage explicit paths (no `git add -A`):
  ```
  git add Cargo.toml Makefile
  git add edge/host/Cargo.toml
  git add edge/host/src/voice/mod.rs
  git add edge/host/src/voice/http_stt.rs
  git add edge/host/src/voice/http_tts.rs
  git add edge/host/src/voice/router.rs
  git add edge/host/tests/http_voice_engines.rs
  ```
  Then commit (separate command):
  ```
  git commit -q -F - <<'EOF'
  feat: [voice] real HTTP STT/TTS engines (faster-whisper + Kokoro sidecars)
  EOF
  ```
