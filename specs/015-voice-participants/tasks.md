# Tasks: Voice as Bus Participants

**Feature Branch:** `015-voice-participants`
**Inputs:** spec.md (4 P1 user stories), plan.md (tech stack, structure), research.md (engine decisions R1–R6), constitution
**Test discipline:** Article I — every behaviour-changing impl task is preceded by a test task. Voice logic is tested headless with `FakeStt`/`FakeTts` + scripted PCM fixtures + a fake registry/dispatch (D-TEST-1/2/3/4); real-device + AEC-on-hardware behavior is a manual checkpoint (needs the workstation).

> **Article IV note:** the **shared free-form intake routing** (capture→STT→transcript→run-targeting→command) lives in **Phase 2 Foundational**, not in any story. Each P1 story (US1 PTT, US2 hot-word, US3 stop, US4 projection) adds an independent modality on that foundation, so each independently yields a viable MVP (resolves challenge H1).

> **Implementation progress (2026-06-21, autonomous loop — headless pure-logic core):** the **decision logic** is implemented + tested green (16 tests, clippy `-D warnings` clean), as inline `#[cfg(test)]` modules matching the voice-module convention (not separate `tests/unit/` files):
> - ✅ **Spoken-cancel matcher** — `edge/host/src/voice/cancel.rs` (`classify_spoken`, FR-005a). Covers T032/T034.
> - ✅ **Projection participant** — `edge/host/src/participants/voice_projection.rs`: the `speakable_text` allowlist (FR-009) **plus the `VoiceProjection` `Agent` impl** (2026-06-22) — subscribes the `run` namespace, speaks the allowlist via the injected `Tts` port, silent otherwise (SC-004), invocation-count tested. Covers **T037 except cpal playback** (device-gated, T038).
> - ✅ **Intake participant** — `edge/host/src/participants/voice_intake.rs`: the `route_transcript`/`IntakeAction`/`to_command` routing (FR-005a/006/007) **plus the `VoiceIntake` `Agent` impl** (2026-06-22) — subscribes `voice`+`run`, dispatches validated `run.start`/`run.steer` and best-effort `run.abort` (spoken cancel; deterministic abort stays on the physical control), tracks the focused run from run snapshots/finishes (EC-007). Covers **T014 + the headless half of T016** except capture→AEC→STT (device-gated). Tested via the bus command-intake (`take_commands`).
> - ✅ **AEC3 (the #1 native risk) — DONE at the DSP level.** **T004 spike PASS**: `webrtc-audio-processing` `bundled` builds on macOS arm64 (needs meson+ninja — now installed). `edge/host/src/voice/aec.rs` (`EchoCanceller`, FR-011a) wraps APM AEC3, **2 tests green headless** (frame_len=160; processes a render+capture frame). Dep added **optional behind the `voice-io` feature** (T001/T003 partial) — default build unaffected (2.3s, clippy clean). Covers T004 + the DSP half of T009/T010. Frame-cadence corrected in research R5 (160-decoupled-from-512, not a 3×160 split).
>
> **VAD (T028) — RESOLVED 2026-06-21 (deep-research):** the `ort` rc.10↔rc.12 conflict is sidestepped by dropping Silero/`ort` for **`earshot` 1.1.0** — pure-Rust embedded-NN VAD, zero `ort`/ONNX (only optional `libm`), MIT/Apache, 256-sample/16 ms frames @ 16 kHz. No published crate provides a tract-based ort-free Silero (every Silero crate pins `ort =2.0.0-rc.10`). **Open: earshot's accuracy claim is unbenchmarked → measure on-device; fallback = vendor `tract-onnx` + Silero v5 `.onnx` by hand.** See research.md R2. (No longer dep-blocked; remaining VAD work is device-gated.)
>
> **Deferred (workstation/device-gated):** cpal capture/playback (real mic/speaker), wake.rs (needs a trained model T042), the participants' **device wiring only** (capture→AEC→STT feeding `voice.utterance_transcribed`; Tts→cpal playback — the bus `handle`/`subscribe` halves are now done), shell shortcuts + bundling + notarization (T041), wake-word training (T042), UI (T040), and the manual gates (T046). These need real audio devices, mic permission, or a trained model.

---

## Phase 1 — Setup (Shared Infrastructure)

- [ ] T001 Add audio/voice deps to `edge/host/Cargo.toml`: `cpal`, `livekit-wakeword`, `earshot` (VAD — pure-Rust, no `ort`; per R2), `webrtc-audio-processing` (feature `bundled`), `ringbuf` (pin versions per research R1–R5).
- [ ] T002 [P] Add scripted-PCM test fixtures (silence, spoken phrase, TTS-echo clip, "stop" clip, "stop wasting tokens" clip) under `edge/host/tests/fixtures/voice/`.
- [ ] T003 [P] Add a `voice-io` cargo feature gate in `edge/host` so the native-audio path compiles out for pure-logic CI lanes.

---

## Phase 2 — Foundational (Blocking Prerequisites)

**No user-story work begins until this phase is complete.**

### Audio plumbing + AEC + bus contracts
- [x] T004 **AEC build spike** — **PASS** (2026-06-21, macOS arm64): `webrtc-audio-processing` `bundled` compiles+links+runs (meson+ninja installed); `num_samples_per_frame()` = 160; real `EchoCanceller` processes a frame headless. Recorded in research R5. Delay calibration deferred to device wiring. **Gate cleared.**
- [ ] T005 [P] Test: cpal capture framing → 16 kHz mono f32, 512-sample frames, in `edge/host/tests/unit/voice_capture.rs`.
- [ ] T006 Implement `edge/host/src/voice/capture.rs` (depends T005).
- [ ] T007 [P] Test: playback emits `SpeechChunk` + exposes the render-reference tap, in `edge/host/tests/unit/voice_playback.rs`.
- [ ] T008 Implement `edge/host/src/voice/playback.rs` (cpal output + pre-output render-reference `ringbuf`) (depends T007).
- [ ] T009 [P] Test: AEC frame adaptation — 512 split into 3×160 for APM, re-framed to 512; reference+capture → cleaned, in `edge/host/tests/unit/voice_aec.rs`.
- [ ] T010 Implement `edge/host/src/voice/aec.rs` (wrap `webrtc-audio-processing`, 10 ms chunking, delay knob; half-duplex fallback if T004 failed) (depends T004, T009).
- [ ] T011 [P] Test: new bus schemas (voice-origin command marker + speakable milestone events) validate against `edge/host/schemas/bus/*.json` (Article X), in `edge/host/tests/unit/voice_schema_validate.rs`.
- [ ] T012 Extend `edge/host/schemas/bus/command.json` + `event.json` (additive-versioned); regenerate `shared/contracts/*.d.ts` (depends T011).
- [ ] T013 [P] Test: participant scaffolding registers on a fake `AgentRegistry`; start/stop/abort route through it (FR-008), in `edge/host/tests/unit/voice_participant_lifecycle.rs`.
- [ ] T014 Add `voice_intake` + `voice_projection` participant skeletons (`Agent` impls) in `edge/host/src/participants/` + register in `edge/shell` (depends T013).

### Shared free-form intake routing (the common path US1 + US2 feed)
- [ ] T015 [P] Test: free-form routing + run-targeting — scripted PCM + `FakeStt` → validated submit-goal when no run focused; steer when a run is focused (FR-006/FR-007); **EC-007** focused-run-terminates-during-STT → treated as new-run goal, in `edge/host/tests/unit/voice_intake_freeform.rs`.
- [ ] T016 Implement the shared free-form intake routing in `edge/host/src/participants/voice_intake.rs`: capture(post-AEC)→`Stt`→transcript→run-targeting→validated command via dispatch (depends T014, T015).

### Cross-cutting guarantees (privacy / schema / errors / toggle)
- [ ] T017 [P] Test: **FR-013** schema-validation at every voice emit site — `FakeDispatch` receives a schema-valid payload; an invalid payload is rejected before emit, in `edge/host/tests/unit/voice_emit_schema.rs`.
- [ ] T018 [P] Test: **FR-012** privacy — a representative run sync via the stub hub (D-TEST-4) transmits **no audio/transcript field** (Article IX verification, `constitution.md:147`), in `edge/host/tests/unit/voice_privacy_sync.rs`.
- [ ] T019 [P] Test: **FR-014** typed error — mic-denied, STT-down, TTS-down each surface a typed `VoiceError` via `VoiceStatus` (no panic/unwrap), 3 cases, in `edge/host/tests/unit/voice_error_surface.rs`.
- [ ] T020 [P] Test: **FR-015** toggle gate + **EC-006** — voice off ⇒ no `AudioChunk` captured and no `FakeTts` invoked; toggling off mid-capture/mid-playback halts both with no submission, in `edge/host/tests/unit/voice_toggle_gate.rs`.
- [ ] T021 Implement the cross-cutting guards: emit-site schema validation, privacy (no audio/transcript on sync), typed-error surfacing, and the toggle gate, across `voice_intake`/`voice_projection`/`capture`/`playback` (depends T017–T020).

**Checkpoint:** audio plumbing, AEC, bus contracts, participant lifecycle, the shared intake routing, and the cross-cutting guards are ready. Story phases can proceed independently.

---

## Phase 3 — User Story 1 — Push-to-talk a goal or steer (Priority: P1) 🎯 MVP

**Goal:** Hold a key, speak, release → transcript submitted as a new run goal (no focused run) or a steer (focused run).
**Independent Test:** PTT "research the voice landscape" with no active run → a new run starts with that goal via validated dispatch.

- [ ] T022 [P] [US1] Test: PTT trigger — key-down starts capture, key-up ends the utterance, which flows through the foundational intake to a command (integration over a fake shortcut + FakeStt), in `edge/host/tests/unit/voice_ptt.rs`.
- [ ] T023 [US1] Implement the PTT global shortcut (configurable, default hold Right-Option) in `edge/shell` → drives `capture` → foundational intake (depends T006, T016, T022).
- [ ] T024 [US1] Emit observability fields (`voice.path=ptt`, `voice.route`, `voice.stt_ms`) per plan §1.3.

**Checkpoint:** PTT goal/steer works end-to-end against FakeStt; Independent Test passes headless.

---

## Phase 4 — User Story 2 — Hands-free hot-word activation (Priority: P1)

**Goal:** Say "Hey Wagner" + a goal hands-free → same submission as PTT.
**Independent Test:** "Hey Wagner, summarize my open runs" with no key held → wake detected, endpointed, submitted via the foundational intake.

- [ ] T025 [P] [US2] Test: wake detection fires on the scripted phrase; the TTS-echo clip routed through AEC does NOT fire (self-trigger suppressed), in `edge/host/tests/unit/voice_wake.rs`.
- [ ] T026 [P] [US2] Test: VAD endpointing bounds the post-wake utterance (speech-end), in `edge/host/tests/unit/voice_vad.rs`.
- [ ] T027 [P] [US2] Implement `edge/host/src/voice/wake.rs` (livekit-wakeword; loads "Hey Wagner" model from app-data) (depends T025).
- [ ] T028 [P] [US2] Implement `edge/host/src/voice/vad.rs` (`earshot`, pure-Rust embedded NN; 256-sample/16 ms frames; validate accuracy on-device, fallback to vendored tract+Silero — research R2) (depends T026).
- [ ] T029 [US2] Wire the wake path: wake-hit → VAD-endpointed capture → **foundational intake (T016)**; degrade to PTT-only if wake/VAD fail to load (depends T016, T027, T028).

**Checkpoint:** hands-free wake → submission works on the same foundational intake; self-echo suppressed via AEC; independent of US1's PTT trigger.

---

## Phase 5 — User Story 3 — Stop a run: physical guarantee + flexible spoken cancel (Priority: P1)

**Goal:** Physical control = deterministic stop (no speech/NLU); spoken cancel = flexible best-effort.
**Independent Test:** trigger the physical abort control with a run active → run cancels via registry-cancel, no speech recognition involved.

- [ ] T030 [P] [US3] Test: physical-abort unit → `registry.cancel(focused_run)` deterministically; no-target → non-destructive notification, in `edge/host/tests/unit/voice_abort_physical.rs`.
- [ ] T031 [P] [US3] **Integration test (SC-003 "every time")**: the shell global-shortcut handler calls `registry.cancel` with **no STT/NLU/LLM component invoked** (assert via spy/fakes), in `edge/host/tests/integration/abort_no_speech_path.rs`.
- [x] T032 [P] [US3] Test: spoken-cancel matcher — short "stop"/"never mind" → cancel; "stop wasting tokens" (trailing goal) → free-form. **DONE** (inline tests in `edge/host/src/voice/cancel.rs`, 7 cases green).
- [ ] T033 [US3] Implement the physical abort global shortcut in `edge/shell` → direct `registry.cancel` (no schema/STT) + no-target notification (depends T030, T031).
- [x] T034 [US3] Implement `edge/host/src/voice/cancel.rs` — best-effort matcher (short utterance, no trailing goal). **DONE** (`classify_spoken`/`SpokenIntent`, clippy clean). Wiring into the live `voice_intake` Agent impl deferred (device-gated).

**Checkpoint:** physical stop is deterministic (proven by the no-speech-path integration test); spoken cancel is flexible best-effort and never fires on trailing-goal phrasings.

---

## Phase 6 — User Story 4 — Spoken responses and milestones (projection) (Priority: P1)

**Goal:** Speak agent transmissions + milestones {complete, stopped, awaiting-approval}; stay silent on everything else.
**Independent Test:** trigger a transmission then a completion → both spoken; a non-allowlisted activity event stays silent.

- [ ] T035 [P] [US4] Test (**SC-004 automated**): allowlist — transmission + each milestone → `FakeTts` invoked; a scripted non-allowlisted event stream → **zero** `FakeTts` invocations, in `edge/host/tests/unit/voice_projection_allowlist.rs`.
- [ ] T036 [P] [US4] Test: barge-in — wake/PTT/utterance detected during playback → TTS ducks/stops and capture proceeds (full-duplex); half-duplex path gates during playback, in `edge/host/tests/unit/voice_barge_in.rs`.
- [ ] T037 [US4] Implement `voice_projection`: subscribe events → allowlist filter → `Tts` → `playback` (depends T014, T035).
- [ ] T038 [US4] Implement barge-in: on detection during playback, duck/stop TTS and capture (ring buffer, no corruption); half-duplex fallback gates during playback (depends T008, T029, T036).
- [ ] T039 [US4] Emit projection observability (`voice.proj_event`, `voice.tts_ms`) per plan §1.3.

**Checkpoint:** allowlisted events spoken, others silent (automated), barge-in ducks playback.

---

## Final Phase — Polish & Cross-Cutting Concerns

- [ ] T040 [P] Extend `edge/ui/app/components/VoiceSettingsPanel.tsx` — wake-arm toggle + listening/wake-armed/speaking indicator (D-A11Y-1: non-color glyph/label, reduced-motion); reducer state + `vitest` in `edge/ui/tests/unit/`.
- [ ] T040a [P] **Headless latency-regression test (SC-006)**: pre-recorded PCM + `FakeStt` with a known delay → assert p95 utterance-end→dispatch ≤ 1.5 s and event→TTS-start ≤ 1.0 s in CI, in `edge/host/tests/unit/voice_latency_budget.rs`.
- [ ] T041 Bundling (`edge/shell`): macOS mic permission (`Info.plist` `NSMicrophoneUsageDescription`, entitlements); ORT `load-dynamic` dylib + rpath fixup; AEC static-link via `bundled`; notarization (research R4). **Manual workstation verification.**
- [ ] T042 Wake-word model: train "Hey Wagner" offline via the livekit synthetic-TTS pipeline; install to app-data (FR-016). **Ops/manual step.**
- [ ] T043 Extend `tests/architecture/` so dependency-direction (Gate VII) + offline-completion (Gate VI) cover the new modules.
- [ ] T044 Update `docs/architecture.md` Voice pillar (close the "local audio I/O is the open gap" §6 note) + AGENTS.md/CLAUDE.md lockstep if touched.
- [ ] T045 Performance pass vs SC-005/006; security pass (transcript-as-untrusted, schema validation at every emit per Article X).
- [ ] T046 Manual gate: `make gui-smoke` + a real-device voice walkthrough (PTT, wake, physical+spoken stop, projection, barge-in). **Needs the workstation.**

---

## Dependencies & Execution Order

- **Setup (1)** → first. **Foundational (2)**: T004 (AEC spike) gates T010 + AEC paths; T015/T016 (shared intake) + T017–T021 (cross-cutting guards) complete the foundation; **blocks all stories**.
- **Stories (3–6) are independent on the foundation** — each adds a modality (PTT trigger / wake trigger / stop / projection) and feeds the foundational intake (T016) or the bus. No story depends on another (resolves H1). After Foundational, US1–US4 implementation can run in parallel.
- **Polish (Final)**: T041/T042/T046 are manual workstation gates; T040a is the automated latency guard.

## Implementation Strategy
- **T004 first** — the AEC build spike is the one decision-flipping unknown; if it fails, the half-duplex fallback reshapes barge-in (T038) before it's built.
- MVP = Setup + Foundational + US1 (PTT). Validate the Independent Test, then layer US2/US3/US4 (each independently mergeable).
