# Phase 0 Research — 015 voice participants

Format per entry: **Decision** / **Rationale** / **Alternatives considered**. Engine picks are
backed by adversarially-verified deep-research (the wake-word/VAD/cpal pass and the AEC pass).

---

## R1 — Wake-word engine

- **Decision:** `livekit-wakeword` (Apache-2.0, v0.1.x, pure-Rust ONNX via **ort-tract**). Models
  (mel spectrogram + speech embedding) compile into the binary; only the custom classifier `.onnx`
  loads at runtime. A custom "Hey Wagner" model is trained offline via the synthetic-TTS pipeline.
- **Rationale:** Apache-2.0, no Python runtime, and the ort-tract path means the wake-word half needs
  **no ONNX-Runtime dylib** (only the VAD pulls ORT in). Custom-trained models are cleanly Apache-2.0
  (avoids openWakeWord's CC BY-NC-SA Google embeddings). [015 wake-word deep-research, 3-0 verified.]
- **Alternatives considered:** Picovoice **Porcupine** — rejected: Rust SDK EOL Jul 2025 + mandatory
  server-validated AccessKey (free tier killed Jun 2026), incompatible with offline/no-account.
  **openWakeWord** — Apache-2.0 but no Rust binding (Python framework); would run ONNX via `ort`
  ourselves. **sherpa-onnx** KWS — Apache-2.0 and Rust-bound, but a heavier C++ lib than the
  pure-Rust livekit path.
- **Caveat:** No verified accuracy benchmark for a custom "Hey Wagner" on Apple Silicon exists; the
  glossy livekit "86% recall / 0.08 FPPH" claim was **refuted 0-3**. Tune in build; the physical
  abort control is the safety floor regardless.

## R2 — VAD (endpointing)

- **Decision:** `voice_activity_detector` 0.2.x (MIT, **Silero VAD V5** via the `ort` crate),
  512-sample frames @ 16 kHz.
- **Rationale:** Substantially more accurate than WebRTC VAD (MCC 0.72 vs 0.41 at 50 ms); frame size
  matches the cpal capture + whisper input. [015 deep-research, 3-0 verified.]
- **Alternatives considered:** WebRTC VAD (lighter, GMM, less accurate in noise); a pure-Rust
  hysteresis-RMS gate (MCC ~0.11 — too crude for always-on, retained only as a degraded fallback).
- **Caveat:** Pulls in the **ONNX-Runtime dylib** (the main bundling cost, R4); pins `ort 2.0.0-rc.10`
  (RC); single maintainer — bus-factor risk.
- **⚠️ INTEGRATION BLOCKER (discovered 2026-06-21, not in the isolated research):**
  `voice_activity_detector 0.2.1` is written against **`ort` rc.10**, but the existing
  **`wagner-tts-sidecar` (Kokoro) already pins `ort ^2.0.0-rc.12`**. Cargo unifies to ONE `ort` per
  workspace (rc.12), and rc.12 broke the API the VAD crate uses (`.view()` on `Shape`) → it does not
  compile. Pinning `ort=rc.10` conflicts with tts-sidecar; bumping the VAD crate isn't possible (0.2.1
  is latest). **`voice_activity_detector` is therefore unusable in this workspace as-is.**
  **Revised VAD options (decide in the device session):** (a) a **pure-Rust / tract** Silero VAD (no
  `ort`, mirrors how livekit-wakeword avoids this via ort-tract) — preferred, sidesteps the ort coupling
  entirely; (b) an rc.12-compatible Silero crate (`silero-vad-rust`?); (c) align tts-sidecar + VAD on one
  `ort` (risky — touches the working Kokoro sidecar); (d) the degraded RMS-gate fallback. **AEC is
  unaffected** — `webrtc-audio-processing` uses no `ort`.

## R3 — cpal capture/playback pattern

- **Decision:** cpal at 16 kHz mono f32, 512-sample (32 ms) frames; mic capture → ring buffer →
  AEC → (wake | PTT) → VAD → STT. TTS playback exposes the rendered signal as the AEC reference (R5).
  Reference implementation: `vox` (MIT/Apache).
- **Rationale:** 16 kHz mono f32 is exactly what whisper.cpp consumes; 512-frame matches Silero V5.
  `vox` validates cpal default-host capture → mpsc → resample-to-16k → frame → VAD → whisper. [015
  deep-research, 3-0 verified for the pattern.]
- **Alternatives considered:** device-native sample rate then downsample at STT (more buffering,
  more drift); larger ~100 ms frames (worse endpointing latency).
- **Open (full-duplex):** single duplex stream vs separate in/out streams + the render-reference tap
  — finalized with the AEC decision (R5).

## R4 — ONNX Runtime + native bundling on macOS arm64

- **Decision:** Use ORT `load-dynamic` (dlopen at startup via `ORT_DYLIB_PATH`); inject the dylib
  with Tauri `bundle.macOS.files`/`frameworks`; run `install_name_tool`/`dylibbundler` rpath fixup
  **before notarization**.
- **Rationale:** `voice_activity_detector` (and any ORT-based AEC) download prebuilt ORT at build time
  by default; the app must ship a controlled, signed dylib. Tauri copies dylibs into
  `Contents/Frameworks` without updating rpath. [015 deep-research, 3-0 verified.]
- **Alternatives considered:** static ORT (heavier, version-locked); default download-binaries (not
  signable/shippable).
- **Caveat:** the rpath workaround rests on an unmerged Tauri PR (#12711) — verify against the pinned
  Tauri version; if merged, the manual step may be unnecessary.

## R5 — Acoustic echo cancellation (AEC)

- **Decision:** **`webrtc-audio-processing`** (tonarino, **BSD-3-Clause**, v2.1.0) with the **`bundled`**
  feature (static C++ link via meson → self-contained `.app`, no system lib). WebRTC APM **AEC3**,
  dual-stream API (`process_render_frame` / `process_capture_frame`). Python-free (C++ + Rust FFI).
- **Integration pattern (verified):** tap the Kokoro **TTS PCM buffer before the cpal output callback**
  → push to a lockless ring buffer (`ringbuf`) → read in the capture path and feed as the APM
  **render/reference** frame, with a fixed latency offset for the speaker→mic round-trip. APM's
  built-in delay estimator runs over this; manual calibration is the fallback. Clean capture output
  then flows to wake/VAD/STT.
- **Rationale:** Only candidate with active maintenance (11 releases, v2.1.0 May 2026), AEC3 quality
  (needed to suppress full-band TTS from an always-on wake detector), permissive license, Python-free,
  and a confirmed macOS arm64 static-bundling story. [015 AEC deep-research, 19 sources, 19/25 claims
  3-vote confirmed.]
- **Alternatives considered:** pure-Rust **`aec3`** (MIT/BSD, v0.3.0) and **`sonora`** (BSD-3, WebRTC
  M145 port) — promising, zero C++ build, but **WIP with no ERLE benchmarks** (fallback if the C++
  build is unacceptable). **`aec-rs`** (SpeexDSP NLMS) / **`fdaf-aec`** — quality ceiling too low for
  TTS self-trigger suppression. **rnnoise/nnnoiseless** — noise suppression only, NOT echo cancellation.
  **sherpa-onnx / cpal** — no AEC.
- **✅ T004 SPIKE RESULT (2026-06-21, macOS arm64):** `webrtc-audio-processing` v2.1.0 `bundled`
  **compiles + links + runs** here. **meson 1.11.1 + ninja 1.13.2 are prerequisites** (installed via
  Homebrew; clang/cmake/pkg-config already present); the meson build produced `libwebrtc-audio-processing-2.a`
  in ~25 s. The docs.rs failure was the Linux docs env only. A real `EchoCanceller` (`voice/aec.rs`) now
  constructs an AEC3 `Processor` and processes a render+capture frame **headless** (2 tests green) — the DSP
  needs no audio device. API: `Processor::new(16000)`, `process_render_frame`/`process_capture_frame`
  (multi-channel; mono = single-channel iterator), `num_samples_per_frame()` → 160.
- **RISKS remaining (carry into tasks):**
  1. ~~docs.rs / bundled build~~ → **RESOLVED** by the T004 spike (needs meson+ninja).
  2. **Frame cadence (corrected):** `num_samples_per_frame()` is **160 (10 ms @ 16 kHz)**, confirmed.
     **512 is NOT a multiple of 160** (512 = 3×160+32) — so NOT a clean 3×160 split. The AEC (160) and
     Silero VAD (512) cadences must be **decoupled by a buffer**: feed AEC 160-sample frames, accumulate
     cleaned output, re-window to 512 for VAD/STT.
  3. **Delay alignment** (speaker→mic round-trip + OS buffer) is the primary operational fragility;
     miscalibration → filter diverges → residual echo → self-triggers. Use APM's delay estimator; add a
     manual-calibration knob.
  4. **Double-talk / wake-recall under residual echo** is unbenchmarked — the critical unknown for
     whether the wake detector self-triggers. Measure during build.
  5. macOS CoreAudio system-tap (`AudioHardwareCreateProcessTap`) was **refuted** as a reliable
     reference source → the app-level TTS-buffer tap is the only confirmed approach.
- **Fallback (recorded in spec FR-011a):** if the bundled build or alignment proves infeasible →
  half-duplex (gate wake/intake during TTS) + the physical-abort override; preserves safety + privacy,
  loses listen-over-TTS. Pure-Rust `aec3`/`sonora` is the intermediate fallback.

## R6 — Physical abort control

- **Decision:** an OS **global shortcut** (Tauri global-shortcut API) → direct `registry.cancel(focused_run)`,
  with no STT/NLU/AEC in the path. This is the council-mandated deterministic stop (spec FR-005).
- **Rationale:** relocates the determinism out of the probabilistic speech path (Whisper is
  non-deterministic on noisy input; AEC is imperfect) — see `spec.md §Clarifications` (council 2026-06-21).
- **Alternatives considered:** PTT-key-on-a-different-gesture (viable; a UI/ergonomics choice for the
  plan); spoken-only cancel (rejected — can never be the guarantee).
