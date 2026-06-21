# Feature Specification: Voice as Bus Participants

**Feature Branch:** `015-voice-participants`
**Created:** 2026-06-21
**Status:** Draft
**Input:** Engineer's original description: "015 voice participants — from specs/015-voice-participants/design.md (012 Phase 2: voice as the first new bus AgentRegistry participant). Close the cpal audio I/O gap (mic capture + speaker playback) and wire voice as intake (mic→STT→Command) + projection (events→TTS→speaker) participants on the 014 AgentRegistry. PTT and hot-word ('Hey Wagner'); hybrid voice→command (control verbs deterministic, else free-form); allowlist speak policy."

> **Spec authoring rules:** WHAT/WHY only; HOW lives in plan.md. Every requirement testable; vague adjectives without quantification fail Article II. No silent defaults — catalog defaults cited in `## Defaults Applied`, everything else `[NEEDS CLARIFICATION]`.

---

## User Scenarios & Testing *(mandatory)*

User stories are ordered by priority. P1 is the MVP — if you implement only User Story 1 you must still ship something the user can use and that delivers the value this feature promises.

### User Story 1 — Push-to-talk a goal or steer (Priority: P1)

The operator holds a designated key, speaks a goal or a steering instruction, and releases. The spoken words are transcribed and submitted: as a new run's goal when no run is active/targeted, or as a steer to the targeted active run. The operator gets voice input without typing, hands on keyboard.

**Why this priority:** Engineer decision (2026-06-21): all four stories are P1 — the slice ships PTT, hot-word, control, and projection together. PTT is the deterministic baseline the other paths reuse (capture → STT → Command).

**Independent Test:** With the voice toggle on and no run active, hold PTT, speak "research the voice landscape", release; observe a new run start with that goal via the validated dispatch — verifies capture → STT → free-form command end-to-end.

**Acceptance Scenarios:**

1. **Given** no active run and the voice toggle is on, **When** the operator holds the PTT key, speaks "research the voice landscape", and releases, **Then** the utterance is transcribed and submitted as a new run goal via the validated command dispatch.
2. **Given** an active run targeted by the operator, **When** the operator PTT-speaks an instruction, **Then** the utterance is submitted as a steer to that run (not a new run).
3. **Given** the voice toggle is off, **When** the operator presses the PTT key, **Then** no audio is captured and no command is emitted.

---

### User Story 2 — Hands-free hot-word activation (Priority: P1)

With the voice toggle on, the operator says the wake word ("Hey Wagner") without touching the keyboard. The system detects the wake word, captures the following utterance until the operator stops speaking, transcribes it, and submits it exactly as the PTT path would. The operator gets fully hands-free input.

**Why this priority:** Engineer decision (2026-06-21): P1 — hands-free activation is core to the voice-first slice, shipped alongside PTT rather than deferred.

**Independent Test:** With the voice toggle on, say "Hey Wagner, summarize my open runs" with no key held; observe wake detection, utterance endpointing, and the same goal submission as the PTT path — verifies wake-word + VAD + capture → STT → command.

**Acceptance Scenarios:**

1. **Given** the voice toggle is on and no PTT key is held, **When** the operator says "Hey Wagner" followed by a goal, **Then** the wake word is detected, the following utterance is endpointed (speech-end detected), transcribed, and submitted identically to the PTT path.
2. **Given** the wake word has not been spoken, **When** ambient speech or noise occurs, **Then** no utterance is captured or submitted (subject to the SC-005 false-accept target, ≤ 1/hour typical ambient).

---

### User Story 3 — Stop a run: physical guarantee + flexible spoken cancel (Priority: P1)

The operator stops the focused/active run two ways: a **physical control** (key/gesture/global shortcut) that cancels instantly with no speech/NLU in the path — the deterministic safety guarantee — and, as a convenience, a **flexible spoken cancel** ("stop", "cancel", "never mind", "knock it off") recognized best-effort on-device and routed to the same cancel action. The operator gets a reliable hard-stop plus natural hands-free phrasing (council 2026-06-21).

**Why this priority:** Engineer decision (2026-06-21): P1 — a deterministic stop is a safety control; it ships with the input paths, not after. The guarantee lives in the physical control so the spoken path can be flexible without risking the safety property.

**Independent Test:** With a run active and targeted, trigger the physical abort control; observe the run cancel via the run-supervision cancel path with no speech recognition involved — verifies the deterministic stop. (Spoken cancel verified separately as best-effort.)

**Acceptance Scenarios:**

1. **Given** an active focused run, **When** the operator triggers the physical abort control, **Then** the run is cancelled immediately via the run-supervision cancel path with no speech/NLU/LLM in the path.
2. **Given** an active focused run, **When** the operator speaks a short cancel phrasing ("stop" / "never mind" / "knock it off") with no trailing goal-like content, **Then** the same cancel action fires (best-effort); **and** when the utterance carries trailing goal content ("stop wasting tokens"), it is treated as free-form, not cancel.
3. **Given** no focused/active run, **When** the operator triggers the physical control or speaks a cancel phrasing, **Then** the system takes no destructive action and surfaces a brief **non-destructive notification** — the voice status indicator briefly reflects "nothing to cancel" (optionally an audible cue).

---

### User Story 4 — Spoken responses and milestones (projection) (Priority: P1)

With the voice toggle on, Wagner speaks aloud: the agent's conversational output, plus a small set of milestones the operator would want hands-free — run complete, run stopped/aborted, and "awaiting your approval" (gate-blocked). Other bus activity is not spoken. The operator gets hands-free awareness of what the agent is doing and when it needs them.

**Why this priority:** Engineer decision (2026-06-21): P1 — spoken output is the other half of a voice-first loop; shipped in-slice so the operator can work hands-free both directions.

**Independent Test:** With the voice toggle on, trigger an agent transmission and then a run completion; observe both spoken (transmission text, then the "run complete" canned phrase) while a non-allowlisted activity event stays silent — verifies the allowlist projection.

**Acceptance Scenarios:**

1. **Given** the voice toggle is on, **When** the agent emits a conversational transmission event, **Then** the projection participant synthesizes and plays it as speech.
2. **Given** the voice toggle is on, **When** a run reaches a speakable milestone (complete, stopped/aborted, gate-blocked/awaiting-approval), **Then** a canned phrase for that milestone is spoken.
3. **Given** a non-allowlisted bus event (fine-grained activity), **When** it is emitted, **Then** the projection participant does NOT speak it.

---

### Edge Cases

- **EC-001 (boundary — empty/silent utterance):** Operator triggers PTT or wake but produces no speech (silence/noise only). Expected: **drop silently** — no command emitted; the listening indicator returns to idle. No audible nack [ASSUMPTION — engineer confirm].
- **EC-002 (concurrency — barge-in):** A wake word or new utterance arrives while the previous utterance is still being transcribed, or while TTS is actively playing. Expected (**full-duplex, AEC available** — R5): AEC lets the detector fire over playback; on detection the system **ducks/stops** the current TTS and captures the new utterance. **Half-duplex fallback** (FR-011a, if the AEC spike T004 fails): barge-in over TTS is **NOT** supported — the wake detector + intake are gated during playback; the **physical abort control (FR-005) remains available and unaffected**. In both modes, an utterance arriving mid-transcription does not drop or corrupt either stream (app-layer ring-buffer + thread strategy).
- **EC-003 (failure — STT/TTS sidecar unavailable):** The whisper or Kokoro sidecar is not running or returns an error mid-request. Expected: a typed error surfaces via the existing `VoiceStatus`/voice-settings UI state (the 010 surface); never a panic. Intake drops the utterance; projection stays silent until the sidecar recovers.
- **EC-004 (failure — microphone permission denied / device absent):** macOS denies microphone access, or no input device exists. Expected: voice input disables, the toggle reflects a disabled state with the reason via `VoiceStatus`/voice-settings UI; the app stays fully usable by text. Re-checking permission re-enables without restart [ASSUMPTION — engineer confirm].
- **EC-005 (malicious/adversarial input — wake-word spoofing / audio injection):** Audio from the app's own TTS output, a video, or another speaker triggers the wake word or a control verb. Expected: the system's **own TTS output** MUST NOT self-trigger the wake detector or a control verb — guaranteed by AEC (FR-011a). External audio (another speaker, a video) triggering the wake word is accepted as in-band for an always-listening assistant and bounded only by wake-word accuracy (FR-017); **no voice-fingerprint/speaker-identity gate in v1** [ASSUMPTION — engineer confirm] — the physical abort control remains the guaranteed stop, and a false external trigger produces at worst a free-form utterance the operator can see and cancel.
- **EC-006 (boundary — toggle-off during active capture/playback):** The operator disables the voice toggle while PTT is held or while TTS is actively playing. Expected: the system **immediately stops audio capture and halts playback**; any in-progress utterance is **not** submitted; no panic. (Covers the FR-015 toggle state-mutation flow.)
- **EC-007 (concurrency — focused run terminates during STT):** The focused run completes, errors, or is cancelled (by any path) while its utterance is still being transcribed. Expected: the returned transcript is treated as a **new-run goal** (the focused target no longer exists); the system MUST NOT submit a steer to a terminal/non-existent run. (Bounds the FR-007 run-targeting race.)
- **EC-008 (boundary — overlong utterance):** PTT is held or speech continues past a **maximum utterance length (default 30 s)** [ASSUMPTION — engineer judgment]. Expected: the system **auto-endpoints at the cap**, transcribes what was captured, and does NOT buffer audio unbounded (the capture ring buffer is bounded). Protects memory and the STT path.

---

## Functional Requirements *(mandatory)*

- **FR-001:** System MUST capture microphone audio from the OS default input device as 16 kHz mono PCM (the format the existing STT consumes).
- **FR-002:** Operators MUST be able to capture an utterance via push-to-talk: capture begins on key-down and the utterance ends on key-up. The PTT trigger is a **configurable OS global shortcut, default = hold Right-Option** [ASSUMPTION — engineer judgment]. **Rationale:** Right-Option is unused by default macOS system shortcuts and rarely bound by IDEs, minimizing conflict; configurable so the operator can rebind.
- **FR-003:** System MUST run an always-on wake-word detector that recognizes a configured wake word ("Hey Wagner"); on detection it MUST capture the following utterance and detect speech-end (endpointing) to bound it.
- **FR-004:** System MUST transcribe a captured utterance using the existing speech-to-text engine and produce a text transcript.
- **FR-005:** System MUST provide a **physical abort control** (a dedicated key / gesture / global shortcut) that cancels the focused (or sole active) run **immediately** via the run-supervision cancel path, with **no speech recognition, NLU, AEC, or LLM in the path**. This is the deterministic safety guarantee for stopping a run (council 2026-06-21).
- **FR-005a:** System SHOULD additionally recognize a **spoken cancel intent** from natural phrasings (e.g. "stop", "cancel", "never mind", "knock it off") via an on-device **best-effort** matcher and route it to the same run-cancel action. This spoken path is an explicit **convenience, not the safety guarantee** (FR-005 is). To limit false aborts, the matcher MUST only fire on a **short utterance with no trailing goal-like content** (e.g. "stop wasting tokens" is treated as free-form, not cancel); the spoken vocabulary is open/flexible rather than a fixed exact-match list.
- **FR-006:** System MUST map a non-control transcript to a free-form command (new-run goal, or steer to the targeted active run) submitted through the validated command dispatch.
- **FR-007:** System MUST route a free-form utterance as a **steer** to the **focused run** when one is focused/selected (or when exactly one run is active); otherwise it MUST start a **new run** with the utterance as the goal. "Focused" reuses the targeted-run concept from the 014 registry.
- **FR-008:** *(Cross-cutting infrastructure — required by US1–US4, not a standalone feature; realizes the 014 `AgentRegistry` participant pattern.)* The voice intake path and the voice projection path MUST each register as supervised participants of the run registry, such that start/stop/abort flow through the same lifecycle as runs. Test coverage: T013/T014.
- **FR-009:** System MUST speak aloud only an allowlisted set of bus events: the agent's conversational transmission output, and the milestones {run complete, run stopped/aborted, run awaiting-approval/gate-blocked}. All other events MUST NOT be spoken.
- **FR-010:** System MUST synthesize allowlisted speech via the existing text-to-speech engine and play it to the OS default output device.
- **FR-011:** System MUST support **full-duplex** barge-in: a wake word, control verb, or new utterance MUST be detectable **while TTS is playing**. On such a detection during playback, the system MUST duck or stop the current TTS playback and capture the incoming utterance. A wake/utterance arriving while a prior utterance is still being transcribed MUST NOT drop or corrupt either stream.
- **FR-011a:** System MUST apply **acoustic echo cancellation (AEC)** so the always-on wake detector and the spoken-cancel recognizer do NOT trigger on the system's own TTS output (no self-trigger / no self-wake). AEC uses **`webrtc-audio-processing` (WebRTC APM AEC3, BSD-3-Clause, bundled/static)** fed the TTS render signal as the reference stream (research R5). If the bundled build or delay alignment proves infeasible, the system MUST fall back to **half-duplex** (gate the wake detector + intake while TTS plays) with the physical abort control (FR-005) still interrupting.
- **FR-012:** Captured audio and transcripts MUST remain local to the edge device; they MUST NOT be sent to the hub or any non-loopback network destination (Article IX). [Applied default: D-DATA-1.]
- **FR-013:** Every command the intake participant emits and every event the projection participant consumes MUST validate against its declared bus schema before use (Article X). [Applied default: D-SEC-3.]
- **FR-014:** System MUST surface a typed error/state (never a panic) when the STT or TTS sidecar is unavailable, the microphone permission is denied, or no audio device is present.
- **FR-015:** Voice capture and projection MUST be gated by the existing in-app voice on/off toggle; when off, no audio is captured and nothing is spoken.
- **FR-016:** System MUST load a custom "Hey Wagner" wake-word model from **app-data** (atomically installed per D-RES-1). The model is trained **offline** via the livekit-wakeword synthetic-TTS pipeline [ASSUMPTION — engineer confirm]; the training run is a one-time build/ops step, out of the runtime path, and produces an Apache-2.0 custom model (avoids the openWakeWord CC BY-NC-SA embeddings).
- **FR-017:** Wake-word false-accept behavior MUST meet the SC-005 target (≤ 1 false-accept/hour typical ambient); recall is tuned during build. The physical abort control (FR-005) remains the guaranteed stop, so a missed/false spoken trigger is non-catastrophic.
- **FR-018:** End-to-end voice latency MUST meet the SC-006 targets (p95 utterance-end → command ≤ 1.5 s; speakable-event → TTS-start ≤ 1.0 s).

---

## Success Criteria *(mandatory)*

- **SC-001:** An operator can speak a goal via push-to-talk and see a corresponding run start (or steer applied) without typing — verified end-to-end.
- **SC-002:** An operator can say "Hey Wagner" + a goal with no keyboard interaction and get the same result as the PTT path.
- **SC-003:** The **physical abort control** cancels the focused/active run **every time** it is triggered, with no speech/NLU/LLM dependency. (The spoken cancel is an explicit best-effort convenience and is not held to the every-time guarantee.)
- **SC-004:** Allowlisted events are spoken and non-allowlisted events are silent — measured as zero spoken non-allowlisted events in a representative session.
- **SC-005:** [ASSUMPTION — engineer judgment, Article II(c)] Wake-word false-accepts ≤ **1 per hour** in typical desktop ambient. **Rationale:** a usability threshold — more than ~1 spurious activation/hour is disruptive; no verified benchmark exists for a custom synthetic-TTS model, so the number is **measured during the T037 build and adjusted if unmet**, and the physical abort control remains the guaranteed stop regardless.
- **SC-006:** [ASSUMPTION — engineer judgment, Article II(c)] **p95 utterance-end → command dispatched ≤ 1.5 s**; **p95 speakable-event → TTS-start ≤ 1.0 s**. **Rationale:** whisper.cpp on M-series runs at RTF ≈ 0.05–0.1 (published whisper.cpp benchmarks), so a ~3 s utterance transcribes in ~0.15–0.3 s, leaving >1 s for AEC/VAD/routing — 1.5 s is a comfortable p95 budget. **Enforced by a headless latency-regression test (T040a)**, not only a manual pass.

---

## Key Entities *(include only when feature involves data)*

- **AudioChunk:** A bounded segment of captured PCM audio (16 kHz mono) representing one utterance, produced by capture and consumed by STT. (Existing voice domain type.)
- **Transcript:** The text result of transcribing an AudioChunk. Routed to either the control-verb path or the free-form command path.
- **Voice Command:** A command emitted by the intake participant — either a deterministic run-cancel (control verb) or a free-form goal/steer (validated dispatch). Shape declared in the bus command contract.
- **Speakable Event (allowlist):** The subset of bus events the projection participant vocalizes — the conversational transmission plus the milestone set. Membership is an explicit allowlist, additive-versioned.
- **Wake-word model:** A custom-trained "Hey Wagner" classifier loaded by the always-on detector from **app-data** (atomic install, D-RES-1); trained **offline** via the livekit-wakeword synthetic-TTS pipeline (FR-016).

---

## Assumptions

- The engineer chose to ship **both** PTT and hot-word activation in this single slice (not phasing wake-word to a follow-up).
- The existing voice engines (whisper.cpp STT, Kokoro TTS), their HTTP contract, sidecar lifecycle, in-app toggle, and model-download remain unchanged and are reused; this feature adds device audio I/O and the bus wiring only.
- The 014 run registry and 013 typed bus contracts exist and are the integration points.
- A custom "Hey Wagner" wake-word model will be trained (synthetic-TTS pipeline); it is not a pre-existing asset.
- The voice engines and detectors run fully on-device / loopback (no cloud).

### Defaults Applied

- `D-DATA-1` — applied to FR-012 (audio + transcripts stay local; only metadata/learnings ever sync).
- `D-SEC-3` — applied to FR-013 (validate all external inputs against a declared schema).
- `D-PROJ-4` — applied to plan scope (edge work continues the Rust host + TypeScript/Tauri stack).
- `D-TEST-1` — applied to testing approach (host tested with cargo; voice uses the existing `FakeStt`/`FakeTts` scripted doubles, analogous to the EngineRunner trait — no real device/CLI in tests).
- `D-TEST-2` — applied to any frontend logic (vitest; pure reducer).
- `D-TEST-3` — applied to schema-validation tests for any new committed bus schema.
- `D-RES-1` — applied to any new persistent state writes (atomic temp-write + rename).
- `D-A11Y-1` — applied to operator-facing voice UI surfaces (state never conveyed by color alone; reduced-motion alternatives). Voice as a modality complements, not replaces, visual state.

### Defaults Overridden

- *(empty)*

---

## Out of Scope

- Windows / Linux audio device support and binary acquisition (macOS arm64 first; structure leaves room — carried 010 boundary).
- Streaming / incremental STT (utterance is transcribed after endpointing, not word-by-word) unless a NEEDS CLARIFICATION resolves otherwise.
- Replacing whisper.cpp or Kokoro, or changing the existing HTTP voice contract.
- Multi-speaker / speaker-diarization, multi-locale wake words (en-US only — D-I18N-1 placeholder).
- The pure-Rust RMS-VAD fallback (named in the design only as a mitigation if ORT bundling proves blocking).
- Any cloud or non-loopback voice processing.

---

## Dependencies

- **cpal (audio capture/playback):** Provides OS audio device access. If the default device is absent or permission denied → FR-014 typed-error path.
- **livekit-wakeword (Apache-2.0):** Provides always-on wake-word detection (pure-Rust ONNX via ort-tract). If the model fails to load → wake path disabled; PTT still works.
- **voice_activity_detector / Silero V5 (MIT):** Provides utterance endpointing. Pulls in the ONNX Runtime dylib (bundling cost). If it fails to load → the **wake/hands-free path disables**, **PTT still works** (key-up bounds the utterance without VAD); a typed state surfaces the degraded mode.
- **whisper.cpp STT sidecar (existing):** Transcription. If down → FR-014 typed error; no transcript produced.
- **Kokoro TTS sidecar (existing):** Speech synthesis. If down → FR-014 typed error; projection silent.
- **ONNX Runtime dylib:** Required by the VAD crate; must be bundled + rpath-fixed + notarized (plan concern).
- **Acoustic echo canceller — `webrtc-audio-processing` (BSD-3-Clause, AEC3, bundled/static):** Required by FR-011a for full-duplex listening over TTS; fed the TTS render signal as reference (research R5). Failure modes: (a) the v2.1.0 bundled build may not compile on macOS arm64 (docs.rs build failed — validate in the first impl spike); (b) render→capture delay miscalibration → residual echo → self-triggers. On failure → half-duplex + physical-abort fallback (no feature loss to safety/privacy, loses listen-over-TTS).
- **014 run registry:** Supervises the two participants and provides the deterministic cancel path. If absent, the feature cannot integrate.
- **013 bus contracts:** Declares the command/event schemas the participants validate against.

---

## Cross-Plugin Surfaces *(crosses plugin boundaries)*

| Plugin / layer | Obligations | Owner |
|----------------|-------------|-------|
| `edge/host` (Rust voice + participants) | cpal capture/playback in `voice/`; wake-word + VAD; intake + projection participants in `participants/`; emit validated commands; consume allowlisted events | Mark (solo) |
| `edge/shell` (Tauri) | Bundle the new native libs/models; manage the ONNX Runtime dylib (load-dynamic + rpath fixup + notarization); macOS mic permission (`NSMicrophoneUsageDescription`, entitlements); PTT global shortcut wiring | Mark (solo) |
| `edge/ui` (React) | Surface voice state (listening / wake-armed / speaking) per D-A11Y-1; respect the existing toggle | Mark (solo) |
| `shared/contracts` | Any new bus command/event shapes for voice intake + speakable milestones (additive-versioned) | Mark (solo) |

---

## Constitution Addenda *(optional)*

- *(none beyond the project constitution; FR-012 restates Article IX for the audio path.)*

---

## Clarifications

Filled in by `/spec clarify`.

### Session 2026-06-21

- Q: Priority ladder for the four voice stories? → A: **All four are P1** — PTT, hot-word, control verbs, and projection ship together in this single slice. (Independent Tests drafted as `[PROPOSED — engineer to confirm]`.)
- Q: Free-form utterance → new run vs steer? → A: **Steer the focused run if one is focused (or exactly one is active); otherwise start a new run.** (FR-007.)
- Q: Echo/self-trigger + barge-in model? → A: **Full-duplex with acoustic echo cancellation** — wake/control work over TTS; on detection, duck/stop playback and capture. Adds an AEC pipeline (FR-011a) with **no engine selected yet** → Phase-0 research item; fall back to half-duplex + PTT override if no viable Rust AEC. (FR-011, EC-002, EC-005.)
- Q: Control-verb design — rigid vocabulary vs flexible "Siri but better"? → A (via **/council** 2026-06-21): **Relocate determinism to a physical abort control** (key/gesture, no speech/NLU) as the safety guarantee; make **spoken cancel a flexible best-effort convenience** (open natural phrasings, short-utterance + no-trailing-goal heuristic); all other free-form flexibility comes from the LLM agent (no second NLU, no rigid list). Resolves FR-005/FR-005a, US3, SC-003. Council consensus: flexibility is already solved by the LLM; abort must not depend on the LLM (and Skeptic+Critic: not on Whisper either).
- Supersedes `design.md §Key design decisions #2`: that brainstorm framing called the spoken-cancel path "deterministic"; the **council decision makes the spoken-cancel path best-effort** and relocates determinism to the physical control (FR-005/005a). The spec is authoritative on this point.
- Batch (proposed-defaults during Phase 3 plan, engineer to confirm): targets SC-005 (≤1 false-accept/hr) + SC-006 (p95 ≤1.5 s dispatch / ≤1.0 s TTS-start) → FR-017/018; PTT = configurable global shortcut (default hold Right-Option, FR-002); wake model in app-data, offline-trained (FR-016); empty utterance → drop silently (EC-001); STT/TTS-down + mic-deny → typed VoiceStatus UI state (EC-003/004); VAD-load-fail → wake disables, PTT still works (Dependencies); no speaker-identity gate in v1 (EC-005); all cross-plugin owners = Mark (solo). **Only open marker: the AEC engine (FR-011a), pending deep-research `wzeslbuvw`.**
