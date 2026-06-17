# Wagner — local development guide

Wagner is a two-layer platform: **edge** (a Rust + Tauri desktop app) and
**hub** (a Deno + Hono always-on peer). You rarely need both running
simultaneously during development — pick the layer you are working on.

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust toolchain | pinned in `rust-toolchain.toml` (1.91.1) | [rustup.rs](https://rustup.rs) |
| Node.js | 20 LTS | [nodejs.org](https://nodejs.org) |
| Deno | 2.x | [deno.com](https://deno.com) |
| Docker | 24+ | [docker.com](https://docker.com) (hub container only) |

Run `make dev-setup` after cloning to verify all tools are present.

```
make dev-setup
```

---

## First-time setup

```bash
# 1. Clone
git clone <repo-url> wagner
cd wagner

# 2. Check toolchains
make dev-setup

# 3. Install Node deps (edge UI + shared workspace)
npm install

# 4. (Optional) Download Rust deps so the first cargo build is fast
cargo fetch
```

No further bootstrapping is required. Secrets (OIDC config) are only needed
when running hub in authenticated mode — see "Hub environment variables" below.

---

## Running each layer

### Hub (Deno + Hono)

Hot-reload dev server on port 8787:

```bash
make dev          # cd hub && deno task dev
```

The hub starts without OIDC config. The `/health` probe works immediately.
Authenticated routes (`/v1/*`) return 401 until the env vars are set — this
is intentional and safe for local work.

**Hub environment variables** (set in your shell or a `.env.hub` file loaded
by your shell):

| Variable | Purpose |
|----------|---------|
| `OIDC_ALLOWED_ISSUERS` | Comma-separated trusted issuers (Google, JumpCloud) |
| `OIDC_CLIENT_ID` | The hub's OIDC `aud` claim |
| `OIDC_ALLOWED_DOMAINS` | Verified email domains (e.g. `adyton.io`) |
| `OIDC_ALLOWED_GROUPS` | Optional IdP group names for access |
| `OIDC_ISSUER_JWKS` | Optional inline JWKS JSON (for offline dev) |

### Edge desktop app

Requires the edge UI to be built first:

```bash
make edge         # build edge/ui/dist → launch Tauri shell
```

For hot-reload UI development (recommended when working on edge/ui):

```bash
# Terminal 1 — vite dev server
npm --prefix edge/ui run dev

# Terminal 2 — Tauri in dev mode (loads vite :1420)
cd edge/shell && cargo tauri dev
```

### Edge UI tests only (headless browser)

```bash
make edge-ui      # vite + Playwright smoke test
```

### Rust edge-host engine (no Tauri)

```bash
make cargo        # full test suite
make clippy       # lint gate
make e2e          # cross-process gate e2e only
```

---

## Test story

| Command | What it runs | Network |
|---------|-------------|---------|
| `make hub` | Unit + contract tests (in-process, no server) | none |
| `make hub-e2e` | E2E: real `Deno.serve` on a random port, local OIDC stub | loopback only |
| `make cargo` | Rust engine unit + integration tests | none |
| `make e2e` | Rust gate E2E (spawns a real subprocess) | none |
| `make ts` | TypeScript unit tests across all workspaces | none |
| `make edge-ui` | Playwright browser smoke test | loopback only |
| `make verify` | Full pre-merge gate (all of the above + edge-build) | minimal |

Run `make verify` before raising a PR. Exit code 0 = all green.

---

## Docker workflow (hub container)

Build and start the hub container:

```bash
make docker-hub         # docker build + docker compose up hub
```

Or manually:

```bash
docker build -t wagner-hub:local hub/
docker compose up hub

# Tail logs
docker compose logs -f hub

# Health check
curl http://localhost:8787/health

# Stop
docker compose down
```

The `HUB_PORT` env var overrides the host port (default 8787):

```bash
HUB_PORT=9000 docker compose up hub
```

See `docker-compose.yml` for the full list of OIDC environment variables
injected into the container. No secrets are baked into the image.

---

## Makefile quick reference

```
make run         # start hub + voice + edge (one-command dev stack)
make down        # stop background hub + voice sidecars
make dev         # hub dev server only (hot-reload)
make edge        # build edge UI → launch Tauri desktop app
make hub         # hub unit + contract tests
make hub-e2e     # hub E2E server tests
make cargo       # Rust engine tests
make clippy      # Rust lint gate
make ts          # TypeScript tests
make typecheck   # TypeScript type-check (no emit)
make edge-build  # build edge UI bundle only
make edge-ui     # headless browser smoke test
make docker-hub  # build hub image + docker compose up
make dev-setup   # verify toolchain prerequisites
make verify      # full pre-merge gate
make voice-up    # start STT+TTS native sidecars on :8771/:8772 (dev stack)
make voice-down  # stop the native voice sidecars (dev stack)
make voice-e2e   # real-sidecar voice smoke tests (needs voice-up first)
make edge-bundle # stage binaries + cargo tauri build → distributable .app
```

---

## Repository layout

```
wagner/
├── edge/
│   ├── host/       Rust engine (wagner-edge-host crate)
│   ├── shell/      Tauri shell (wagner-edge-shell crate)
│   └── ui/         React + Vite frontend (edge/ui/dist served by Tauri)
├── hub/
│   ├── src/        Deno + Hono server (main.ts, app.ts, auth/, routes/, discovery/)
│   └── tests/      unit/, contract/, e2e/
├── docs/
│   ├── adr/        Architecture Decision Records
│   └── spec/       Constitution + specs
├── Makefile        Developer targets
├── Dockerfile      (hub/) multi-stage Deno build
└── docker-compose.yml  hub service definition
```

---

## Voice sidecars (STT + TTS) — native engines

The voice pillar's `HttpStt` / `HttpTts` adapters call two OpenAI-compatible
HTTP services on loopback. Both are native binaries — no Docker required.

| Service | Port | Binary | Endpoint |
|---------|------|--------|----------|
| STT | 8771 | `whisper-server` (whisper.cpp, `brew install whisper-cpp`) | `POST /v1/audio/transcriptions` |
| TTS | 8772 | `wagner-tts-sidecar` (built from `edge/tts-sidecar` in this repo) | `POST /v1/audio/speech` |

### Two paths: dev stack vs. app-managed

**Dev stack path** — for developing/testing the voice engines directly:

```bash
make voice-up      # download models if missing → build TTS sidecar if missing → spawn both
make voice-e2e     # run the #[ignore] real-sidecar smoke tests
make voice-down    # stop both sidecars
```

`make run` also boots these automatically alongside hub and the edge app (see
"One-command dev stack" below).

**App-managed path** — the shipped desktop app manages its own sidecars.
When a user enables voice via the **in-app toggle** (TopBar), the app:

1. Checks whether voice models are present in app-data; if not, prompts the
   user to open the voice settings panel to download them first.
2. Spawns both native sidecars via `tauri-plugin-shell` once models are ready.
3. Health-waits `/health` on both ports before flipping the voice state to
   `on`.
4. Kills both sidecars when voice is disabled.

Operators do **not** run `make voice-up` for the shipped app path — the toggle
handles the full lifecycle.

### In-app voice toggle

The **TopBar** shows a voice toggle button that reflects four states:

| State | Meaning |
|-------|---------|
| `off` | Voice disabled; sidecars not running |
| `starting` | Sidecars spawning; health-check in progress |
| `on` | Both sidecars healthy; STT + TTS active |
| `error` | A sidecar failed to start; details in the voice settings panel |

Clicking the toggle calls `voice_set_enabled(true/false)` via `bridge.ts`
(`voiceStatus` / `voiceSetEnabled`), which maps to the `voice_status` /
`voice_set_enabled` host commands.

### Voice settings panel + model download

Models are **not bundled** in the `.app` — they download on first enable into
app-data (≈ 165–240 MB total). The voice settings panel shows per-model
progress and lets the user trigger or re-trigger a download.

| Model file | Size | Source |
|------------|------|--------|
| `ggml-tiny.en.bin` (STT) | 74 MB | `ggerganov/whisper.cpp` on Hugging Face |
| `kokoro-v1.0.onnx` (TTS, q8) | ~92 MB | `onnx-community/Kokoro-82M-v1.0-ONNX` on HF |
| `voices-v1.0.bin` (TTS voices) | 27 MB | `thewh1teagle/kokoro-onnx` GitHub releases |

Progress states per model: `absent` → `downloading` → `verifying` → `ready`
(or `failed`, with a retry button). `voice_set_enabled(true)` is blocked with a
"models not ready" prompt until all models reach `ready`.

For the **dev stack**, `make voice-up` downloads the same models into
`~/.cache/wagner-voice/models/` (separate from app-data). Override with
`WAGNER_VOICE_HOME=/your/path make voice-up`.

### Prerequisites (dev stack only)

- `brew install whisper-cpp` (provides `whisper-server` on your PATH).
- Rust toolchain (already required for `cargo` targets) — the TTS sidecar
  builds automatically the first time `make voice-up` runs.

### Round-trip verification (dev stack)

```bash
make voice-up

# synthesize text → WAV
curl -s -X POST http://127.0.0.1:8772/v1/audio/speech \
  -H 'Content-Type: application/json' \
  -d '{"model":"kokoro","input":"hello from Wagner","voice":"af_bella"}' \
  -o /tmp/s.wav

# transcribe the WAV back
curl -s -X POST http://127.0.0.1:8771/v1/audio/transcriptions \
  -F 'file=@/tmp/s.wav;type=audio/wav' -F 'model=whisper-1'
# → {"text":"Hello from Wagner."}

make voice-down
```

**Note on `make voice-e2e`:** the smoke tests assert *no panic*, not a specific
transcript — `stt_real_sidecar` feeds synthetic silence which whisper rejects
with an error, and the test tolerates that. The round-trip above proves the
full pipeline with real audio.

---

## One-command dev stack

`make run` boots all three layers (hub + voice + edge) with a single command:

```bash
make run    # starts hub + voice sidecars, then launches the edge app (foreground)
make down   # stops the backgrounded hub + voice sidecars
```

The edge app launches in the foreground (blocking). When you are done, quit the
app window (or Ctrl-C), then run `make down` to stop the background services.

The hub logs to `/tmp/wagner-hub.log`. Voice sidecar logs land in
`~/.cache/wagner-voice/run/stt.log` and `~/.cache/wagner-voice/run/tts.log`.

---

## Bundling the app

`make edge-bundle` builds a distributable `.app` containing both native voice
binaries:

```bash
make edge-bundle   # stages binaries → cargo tauri build → .app bundle
```

The Tauri `bundle.externalBin` configuration (in `tauri.conf.json`) declares
the two target-triple-suffixed binaries (`whisper-server` and
`wagner-tts-sidecar`). The resulting `.app` is self-contained for the binaries;
**models are not bundled** — the user downloads them on first voice-enable via
the in-app settings panel.

To verify a bundle from scratch (no dev binaries on PATH, no `make voice-up`):

1. Build: `make edge-bundle`
2. Launch the built `.app`
3. Enable voice — the settings panel prompts for model download
4. Download completes → enable voice → TTS → STT round-trip confirms the
   bundled binaries work independently of the dev stack

---

## Native GUI smoke (macOS)

`make gui-smoke` runs a best-effort native-window check for the assembled Tauri
desktop shell. It is **not** part of `make verify` or `make accept` — it
requires a physical or virtual display and macOS Accessibility permission that
are unavailable in headless CI environments.

### Why not tauri-driver?

[tauri-driver](https://tauri.app/reference/webdriver/) is the official
WebDriver bridge for Tauri apps; however, its documentation explicitly notes
that it is **not supported on macOS** because the embedded WKWebView does not
expose a WebDriver session endpoint (unlike the Chromium-based WebView2 on
Windows). There is no macOS-compatible Tauri UI testing driver at this time.

### What `make gui-smoke` does

1. Builds the edge UI bundle (`make edge-build`)
2. Launches the Tauri shell in the background:
   `cargo run -p wagner-edge-shell --features custom-protocol`
3. Polls macOS System Events via `osascript` until a window whose title
   contains "Wagner" is visible (or a 45-second timeout elapses)
4. Screenshots the display with `screencapture`
5. Quits the app and cleans up

Screenshots land in `edge/ui/.native-shots/`.

### Prerequisites

| Requirement | Notes |
|-------------|-------|
| macOS | Script is Darwin-only; exits immediately on other OSes |
| Display | A physical monitor or a virtual display server (e.g. Xvfb is not supported on macOS; use a real or VNC display) |
| Accessibility permission | Grant in **System Settings → Privacy & Security → Accessibility** for the terminal app running `make gui-smoke` |

### Expected failure modes

| Symptom | Likely cause |
|---------|-------------|
| `osascript failed — Accessibility permission may not be granted` | Terminal not in the Accessibility allowlist |
| `Wagner window did not appear within 45s` | No display attached, or cargo/Rust build time exceeded the timeout (first build can be slow — try increasing `LAUNCH_TIMEOUT` in the script) |
| `screencapture failed` | Terminal not granted Screen Recording permission |

If `make gui-smoke` fails in your environment, the failure is non-fatal for
development — the headless `make accept` gate (`edge-ui` Playwright journey) is
the canonical acceptance check.

---

## Commit conventions

Commit messages follow `type: [scope] description` (enforced by pre-tool hook):

```
feat: [hub] add vault-sync routes
fix: [edge] correct TTL clamping in discovery registry
test: [e2e] add whoami round-trip coverage
docs: [dev] update prerequisites table
```

Allowed types: `feat fix docs refactor test perf build ci chore`

Do not use `type(scope):` (parenthesis) format — the hook will block it.
Do not include AI tool names or attribution in commit messages.
