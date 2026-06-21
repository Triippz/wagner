# Implementation Plan: Voice as Bus Participants

**Feature Branch:** `015-voice-participants`
**Date:** 2026-06-21
**Spec:** [spec.md](./spec.md)
**Constitution:** `docs/spec/constitution.md` v0.1.0

This plan describes HOW the feature will be built. The WHAT lives in spec.md. Tasks live in tasks.md.

## Summary

Close the voice audio-I/O gap and put voice on the bus as two registry-supervised participants. Capture mic audio with `cpal` (16 kHz mono f32), gate utterances by push-to-talk OR an always-on wake word (`livekit-wakeword`) endpointed by Silero VAD (`voice_activity_detector`), and run **acoustic echo cancellation** (engine TBD — Phase-0 research) so the always-on listener doesn't self-trigger on the app's own TTS. Transcribe with the existing whisper.cpp `Stt`; route the transcript via a hybrid policy — a **physical abort control** is the deterministic stop, a flexible best-effort matcher catches spoken cancel, and everything else is a free-form goal/steer `Command` through the validated dispatch. A symmetric **projection** participant speaks an allowlist of events via the existing Kokoro `Tts` over `cpal` playback. Existing voice engines, sidecars, lifecycle, and toggle are reused unchanged.

---

## Technical Context

| Field | Value |
|-------|-------|
| Language / Version | Rust (Edition 2021, `edge/host` + `edge/shell`); TypeScript strict (`edge/ui`, `shared`) |
| Primary Dependencies | `cpal` (audio I/O); `livekit-wakeword` (Apache-2.0, wake word); `voice_activity_detector` 0.2.x (MIT, Silero V5 VAD, `ort`); `webrtc-audio-processing` (BSD-3-Clause, WebRTC APM AEC3, `bundled`/static — research R5); `ringbuf` (render-reference ring); existing `HttpStt`/`HttpTts` + sidecars |
| Storage | None new for runtime. Wake-word model + ONNX/AEC assets stored under app-data (atomic write, D-RES-1). Run state unchanged (event log). |
| Testing | `cargo test` (host, with `FakeStt`/`FakeTts` + scripted audio fixtures); `vitest` (ui reducer/state); `bats` (bundle/install); schema-validation tests (Article X) |
| Target Platform | macOS arm64 first (Tauri desktop); Linux/Windows audio deferred (spec Out of Scope) |
| Project Type | Multi-package monorepo addition (`edge/host`, `edge/shell`, `edge/ui`, `shared`) |
| Performance Goals | [ASSUMPTION — engineer confirm; Article II]: wake-detect ≤ 300 ms after wake-word end; p95 utterance-end → command dispatched ≤ 1.5 s (whisper-dominated); p95 speakable-event → TTS-start ≤ 1.0 s; always-on listen (wake+VAD+AEC) ≤ 15% of one M-series core; wake false-accept target ≤ 1/hour typical ambient (no verified benchmark — tune in build) |
| Constraints | Fully local / offline (no cloud NLU, no hub round-trip); audio + transcripts never leave edge (Article IX); subscription CLIs only, no metered API key (Article VI) |
| Scale / Scope | Single operator, single machine; ~1 active focused run targeted by voice at a time; 4 new source modules + 2 participants |

> Unknowns marked `NEEDS CLARIFICATION` are resolved by Phase 0 research (AEC engine is the one real blocker).

---

## Constitution Check

- [x] **Gate I — Test-First:** tasks.md will list each test task ID before its implementation task (enforced in Phase 4; voice uses `FakeStt`/`FakeTts` + scripted PCM fixtures so the full path is testable headless — D-TEST-1).
- [x] **Gate II — Evidence-Driven:** SC-005/006 are Article II(c) assumptions with **stated rationale** (whisper.cpp RTF benchmark; usability threshold) + a **headless latency-regression test (T040a)**. Challenger H2 (FR-002 vague-deferral) resolved with a default + rationale.
- [~] **Gate III — CRITICAL-Resolved:** Challenge round 1 disposed (6 CRITICAL + 5 HIGH accepted + fixed). Spec is `[NEEDS CLARIFICATION]`-free. Final confirmation pending spec-validator (Phase 6).
- [x] **Gate IV — Independent MVP:** All four P1; Independent Tests **confirmed** (no `[PROPOSED]`). Independence is **structural** — the shared intake is Foundational (T015/T016), so each story is independently a viable MVP (challenge H1 resolved).
- [x] **Gate V — Simplicity:** 5 external engines (cpal, livekit-wakeword, voice_activity_detector, webrtc-audio-processing, ringbuf) + ORT, each with a Complexity Tracking entry below; no new project/crate.
- [x] **Gate VI — Edge-Autonomy:** All voice processing is local (sidecars on loopback + on-device detectors); no hub round-trip on any voice path; no metered API key. An offline path already holds.
- [x] **Gate VII — One-Directional Dependency:** New code lives in `edge/host` + `edge/shell` + `edge/ui` + `shared`; nothing in the capability library imports it. Dependency-direction test (`tests/architecture/dependency-direction.test.ts`) continues to guard. Internal rule shell→host→voice preserved (voice never depends up).
- [x] **Gate VIII — Event-Sourced:** Voice intake emits validated `Command`s into dispatch; resulting run-state changes fold through the existing pure reducer. Projection *reads* the event stream. No run-state mutation outside the fold. Voice's own enable/listening status is ephemeral UI state, not run state.
- [x] **Gate IX — Privacy-Boundary:** Audio frames + transcripts stay on-edge (FR-012); the hub-sync payload is untouched (no audio/transcript field added). Default path is privacy-preserving.
- [x] **Gate X — Schema-Validated:** Every new voice `Command` emitted and `Event` consumed validates against a declared bus schema before use (FR-013); new schema files get CI validation tests.

---

## Project Structure

```
edge/host/src/voice/            # existing voice domain (Stt/Tts ports, pipeline, router, manager)
├── capture.rs                  # NEW — cpal mic capture → 16 kHz mono f32, 512-sample frames, ring buffer
├── playback.rs                 # NEW — cpal speaker playback of Tts SpeechChunk; exposes render reference for AEC
├── wake.rs                     # NEW — livekit-wakeword always-on detector ("Hey Wagner")
├── vad.rs                      # NEW — voice_activity_detector (Silero V5) endpointing
├── aec.rs                      # NEW — acoustic echo cancellation (engine per Phase-0 research); render+capture → clean capture
├── cancel.rs                   # NEW — best-effort spoken-cancel matcher (short utterance, no trailing goal)
├── mod.rs                      # extend public surface
└── (existing: tts, stt, http_stt, http_tts, manager, router, pipeline, types, models)

edge/host/src/participants/     # existing bus participants (slack.rs, scheduler.rs)
├── voice_intake.rs             # NEW — Agent participant: capture→AEC→(wake|PTT)→VAD→STT→Command (free-form|spoken-cancel)
└── voice_projection.rs         # NEW — Agent participant: subscribe allowlisted Events → Tts → playback (duck on barge-in)

edge/host/schemas/bus/          # existing bus schemas (command.json, event.json, ...)
└── (extend command/event taxonomy: voice goal/steer command reuse; speakable milestone events; additive-versioned)

shared/contracts/               # regenerated TS contracts for any new/extended command+event shapes

edge/shell/src/                 # existing Tauri shell
├── (commands.rs / lib.rs)      # register voice_intake + voice_projection participants on the AgentRegistry
├── voice_lifecycle.rs          # existing sidecar lifecycle (reused); extend for AEC asset if needed
└── (NEW) physical-abort global shortcut → registry.cancel; macOS mic permission (Info.plist NSMicrophoneUsageDescription, entitlements); AEC/ORT dylib bundling + rpath fixup + notarization

edge/ui/app/                    # existing React surface
├── components/VoiceSettingsPanel.tsx   # extend: wake-word arm toggle, listening/speaking indicator (D-A11Y-1, non-color)
└── (reducer/state)             # voice status projection (vitest)
```

**Structure Decision:** Extend the existing `voice/` domain and `participants/` directory rather than create a new crate — the voice pillar already lives in `edge/host`, and the registry/participant pattern from 014 is the integration seam. No new project (Gate V satisfied structurally; the engine additions are tracked below). Native device + bundle concerns stay in `edge/shell` per the established split (engines/lifecycle in shell, domain in host, dependency one-way).

---

## Phase 0 — Research

Findings persisted to [research.md](./research.md).

1. **AEC engine selection** — ✅ RESOLVED (research R5): `webrtc-audio-processing` (WebRTC APM AEC3, BSD-3-Clause, `bundled`/static). **First impl task is a spike** to validate the bundled build on macOS arm64 (docs.rs build of v2.1.0 failed) + confirm the 10 ms/160-sample APM frame requirement + delay alignment. Fallback: half-duplex + physical-abort (FR-011a).
2. **livekit-wakeword integration** — Confirm Python-trained `.onnx` → Rust `ort-tract` inference path for a custom "Hey Wagner"; model storage under app-data; custom-model training workflow (synthetic TTS) and its licensing (custom-trained = Apache-2.0). (From 015 deep-research; verify the training→inference handoff.)
3. **cpal full-duplex pattern** — Single duplex stream vs separate input/output streams; render-reference tap for AEC; frame alignment (512-sample / 32 ms) and ring-buffer/backpressure; resampling device-native → 16 kHz. Reference: `vox`.
4. **ONNX Runtime + AEC bundling on macOS arm64** — `load-dynamic` + `ORT_DYLIB_PATH`; Tauri `bundle.macOS.files`/`frameworks`; rpath fixup (`install_name_tool`/`dylibbundler`); notarization. Confirm against the pinned Tauri version (PR #12711 status).
5. **Physical-abort control** — Global shortcut vs PTT-key-on-different-gesture; Tauri global shortcut API; ensure the path reaches `registry.cancel` with no STT/NLU.

---

## Phase 1 — Design & Contracts

### 1.1 Data Model

- **AudioChunk** (existing): 16 kHz mono f32 PCM for one utterance. Produced by `capture` (post-AEC), consumed by `Stt`.
- **RenderFrame** (new, transient): the TTS playback signal tapped from `playback`, fed as the AEC reference. Not persisted.
- **Transcript** (existing): STT text; routed to spoken-cancel matcher or free-form command.
- **VoiceCommand** (new, on the bus): either a free-form goal/steer (validated dispatch) or — only via the *physical* control — a run-cancel. Reuses existing run command shapes where possible; additive-versioned.
- **SpeakableEvent allowlist** (new): the membership set {agent transmission, run complete, run stopped/aborted, run awaiting-approval}. Encoded in the projection participant, not a new persisted entity.
- **WakeModel** (new asset): custom "Hey Wagner" `.onnx` classifier under app-data; loaded at startup; atomic install (D-RES-1).

### 1.2 Interface Contracts

- **Voice intake → dispatch:** emits existing run commands (submit-goal / steer / cancel) — no new command *kind* if the existing taxonomy suffices; if a voice-origin marker is needed it is an additive field (013 additive-versioning). Validated against `command` schema before emit (Article X). Contract tests precede impl (Gate VII/Integration-First).
- **Bus → voice projection:** subscribes to the event stream; filters to the allowlist; never emits. Consumes `event` schema-validated payloads.
- **Participant lifecycle:** both implement the `Agent` trait and register on the `AgentRegistry` (014) — start/stop/abort via the registry, same as runs.
- **Physical-abort shortcut (shell):** OS global shortcut → direct `registry.cancel(focused_run)`; no schema/STT in path (it's a control input, not a bus payload).

### 1.3 Cross-Cutting Concerns

#### Observability
- **Log fields:** `voice.path` (ptt|wake), `voice.wake_score`, `voice.vad_ms`, `voice.stt_ms`, `voice.transcript_len`, `voice.route` (cancel-physical|cancel-spoken|goal|steer), `voice.aec_active`, `voice.proj_event`, `voice.tts_ms`.
- **Metrics:** `voice_utterance_total{path,route}`, `voice_wake_false_accept_total`, `voice_stt_latency_seconds`, `voice_tts_latency_seconds`, `voice_barge_in_total`.
- **Trace spans:** `voice.capture`, `voice.aec`, `voice.wake`, `voice.vad`, `voice.stt`, `voice.dispatch`, `voice.projection.tts`.

#### Security
- **Trust boundary:** the microphone is an external input; transcripts are untrusted text. Every emitted command validates against schema (Article X) before dispatch. No transcript is executed as code.
- **Authentication / Authorisation:** voice commands carry the operator identity (D-IDENT-1) like any command; the dispatch authorize step (014 challenge C2) is unchanged. v1 has no voice-fingerprint/speaker-identity gate (spec EC-005 open marker — confirm).
- **Secrets:** none new; engines are local. No API key (Article VI).
- **Privacy:** audio + transcripts never leave the edge (Article IX); not synced.

#### Failure Modes
- **STT/TTS sidecar down:** typed error → UI state; never panic (FR-014). Intake drops the utterance; projection stays silent.
- **Mic permission denied / no device:** voice disables with a typed state; app stays usable (FR-014).
- **AEC (`webrtc-audio-processing`):** render reference tapped pre-cpal-output via a `ringbuf`; APM processes **10 ms (160-sample) frames**, so each 512-sample pipeline buffer is split **3×160** for AEC then re-framed to 512 for VAD. Render→capture delay uses APM's estimator + a manual-calibration knob. If the bundled build fails on macOS arm64 (validated in the first spike) or alignment diverges → **half-duplex + physical-abort fallback** (wake/VAD gated during TTS playback).
- **Barge-in:** on wake/PTT/utterance during playback, duck/stop TTS, capture; in-flight STT not corrupted (ring buffer). 
- **Wake-word false-accept:** bounded by the model + threshold; the physical control remains the only guaranteed stop, so a missed/false spoken cancel is non-catastrophic.

---

## Complexity Tracking

| Violated Gate | Why Needed | Simpler Alternative Rejected Because |
|---------------|------------|--------------------------------------|
| Gate V — new engine `cpal` | Real mic/speaker device I/O; 010 explicitly deferred it; no audio without it | No stdlib audio; the existing HTTP engines transcribe files, not devices |
| Gate V — new engine `livekit-wakeword` | Hot-word activation (engineer-required); pure-Rust ort-tract avoids the ONNX-Runtime dylib | Exact-match-on-STT can't do always-on; sherpa-onnx KWS is a heavier C++ lib (015 deep-research) |
| Gate V — new engine `voice_activity_detector` (Silero V5) | Utterance endpointing after wake; accurate (MCC 0.72 vs WebRTC 0.41) | RMS-gate VAD MCC ~0.11 — too crude for always-on (015 deep-research) |
| Gate V — new engine `webrtc-audio-processing` (AEC3) + ONNX Runtime dylib | Full-duplex without self-trigger (engineer chose full-duplex); AEC3 is the only maintained permissive Rust-callable engine with the needed quality; ORT required by the VAD crate | Half-duplex avoids AEC but loses listen-over-TTS — recorded fallback; pure-Rust aec3/sonora are WIP/unbenchmarked |
| Gate IV — all four stories P1 | Engineer decision: ship the full voice loop in one slice. **Independence is satisfied structurally**: the shared free-form intake routing is **Foundational** (tasks T015/T016), so each P1 story adds an independent modality (PTT trigger / wake trigger / stop / projection) and is independently a viable MVP — no story depends on a sibling (resolves challenge H1) | A strict P1→P4 ladder was offered and declined; "both now" is the stated product intent |

---

## Optional Artifacts

- [x] `research.md` — Phase 0 findings (AEC + wake-word + cpal + bundling decisions).
- [ ] `data-model.md` — not needed (entities are simple, captured above).
- [ ] `contracts/*.yaml` — bus schemas live in `edge/host/schemas/bus/`; no separate IDL artifact.
- [ ] `quickstart.md` — defer to implementation.
