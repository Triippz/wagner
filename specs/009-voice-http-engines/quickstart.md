# Quickstart: Voice HTTP Engines — HttpStt / HttpTts

## Story P1-A: HttpStt transcribes audio via faster-whisper sidecar

**Independent Test:** Given a mock server returning `{"text":"hello world"}`, `HttpStt::transcribe` returns `Ok(Transcript { text: "hello world", .. })` without any real sidecar.

**Automated (hermetic — no sidecars needed):**
```bash
cargo test -p wagner-edge-host --test http_voice_engines stt
```

**Optional real-sidecar smoke (requires faster-whisper-server running on 8771):**
```bash
make voice-e2e
```

## Story P1-B: HttpTts synthesises text via Kokoro sidecar

**Independent Test:** Given a mock server returning four raw bytes, `HttpTts::synthesise("greet the operator")` returns `Ok(SpeechChunk { bytes: [0xDE,0xAD,0xBE,0xEF], .. })`.

**Automated (hermetic):**
```bash
cargo test -p wagner-edge-host --test http_voice_engines tts
```

**Optional real-sidecar smoke (requires Kokoro-FastAPI running on 8772):**
```bash
make voice-e2e
```

## Story P1-C: VoiceRouter::default_http wires HTTP engines as `"local"`

**Independent Test:** `VoiceRouter::default_http(...)` compiles and returns a router that routes the `"local"` tag.

**Automated (hermetic):**
```bash
cargo test -p wagner-edge-host --test http_voice_engines router
```

## Full hermetic suite

```bash
cargo test -p wagner-edge-host --test http_voice_engines
```

All tests must pass with no sidecars running. `#[ignore]`-marked smoke tests are skipped automatically.

## Full verify gate

```bash
make verify
```

Must exit 0. Runs clippy, cargo tests, shell checks, TypeScript typecheck, edge-build, hub.
