# Validation Report: Voice as Bus Participants (015)

**Validator:** spec-validator agent (read-only)
**Generated:** 2026-06-21
**Spec version:** git HEAD (branch `015-voice-participants`)
**Constitution:** `docs/spec/constitution.md` v0.1.0

> Read-only structural review. The validator does not propose substantive changes; it reports whether the spec is structurally complete and consistent enough to advance to implementation.

---

## Verdict

**BLOCKED**

Open blockers (detail in §Open Blockers):

1. **CHK001–CHK032 checklist pass rate = 0%** — all 32 items are unchecked. The checklist has not been signed off by the engineer.
2. The checklist failure is the sole blocker. All other passes return zero CRITICAL findings, 100% FR/SC coverage, all constitution gates pass, no `[NEEDS CLARIFICATION]` markers survive in spec body text, and all Independent Tests are specific and confirmed.

---

## Coverage Matrix

Every functional requirement (FR-NNN) and success criterion (SC-NNN) must map to at least one task. Items with zero mapped tasks are CRITICAL.

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| FR-001 | Mic capture — 16 kHz mono PCM | T005, T006 | OK |
| FR-002 | PTT — configurable global shortcut, default hold Right-Option | T022, T023 | OK |
| FR-003 | Always-on wake-word detector + endpointing | T025, T026, T027, T028, T029 | OK |
| FR-004 | Transcribe captured utterance via STT → text transcript | T015, T016 | OK |
| FR-005 | Physical abort control — deterministic, no STT/NLU/LLM | T030, T031, T033 | OK |
| FR-005a | Spoken cancel — best-effort flexible matcher | T032, T034 | OK |
| FR-006 | Map non-control transcript → free-form command via validated dispatch | T015, T016 | OK |
| FR-007 | Route utterance as steer (focused run) or new-run goal (else) | T015, T016 | OK |
| FR-008 | Participants register on AgentRegistry; lifecycle via registry | T013, T014 | OK |
| FR-009 | Speak only allowlisted bus events; all others silent | T035, T037 | OK |
| FR-010 | Synthesize speech via TTS engine → OS output device | T007, T008, T037 | OK |
| FR-011 | Full-duplex barge-in: wake detectable during TTS; duck/stop on detect | T036, T038 | OK |
| FR-011a | AEC so wake/spoken-cancel do not self-trigger on own TTS | T004, T009, T010 | OK |
| FR-012 | Audio + transcripts stay local; never sent to hub or non-loopback | T018 | OK |
| FR-013 | Every emitted command / consumed event validates against bus schema | T011, T017 | OK |
| FR-014 | Typed error (never panic) for sidecar down, mic denied, no device | T019 | OK |
| FR-015 | Voice gated by in-app toggle — capture and projection off when toggle off | T020 | OK |
| FR-016 | Load custom "Hey Wagner" wake-word model from app-data (atomic) | T027, T042 | OK |
| FR-017 | Wake false-accept meets SC-005 target (≤1/hour typical ambient) | T025, T040a, T045 | OK |
| FR-018 | End-to-end latency meets SC-006 targets (p95 ≤1.5 s dispatch; ≤1.0 s TTS-start) | T040a, T045 | OK |
| SC-001 | PTT speak goal → run starts (or steer applied) without typing, verified end-to-end | T022, T023, T015, T016 | OK |
| SC-002 | "Hey Wagner" + goal → same result as PTT, no keyboard | T025, T026, T027, T028, T029 | OK |
| SC-003 | Physical abort cancels focused run every time, no speech/NLU/LLM dependency | T030, T031, T033 | OK |
| SC-004 | Allowlisted events spoken; non-allowlisted silent; zero non-allowlisted in session | T035, T037 | OK |
| SC-005 | Wake false-accepts ≤1/hour typical ambient | T040a, T045 | OK |
| SC-006 | p95 utterance-end→dispatch ≤1.5 s; p95 speakable-event→TTS-start ≤1.0 s | T040a | OK |

**Coverage: 26/26 requirements covered (100%). Zero CRITICAL coverage gaps.**

---

## Unmapped Tasks

Tasks with no direct FR/SC/user story mapping. Engineer review recommended.

| Task ID | Description | Phase | Severity |
|---------|-------------|-------|----------|
| T001 | Add audio/voice deps to `edge/host/Cargo.toml` | Setup | LOW — prerequisite infrastructure, no standalone FR |
| T003 | `voice-io` cargo feature gate for CI lanes | Setup | LOW — CI hygiene, not a functional requirement |
| T024 | Emit observability fields for PTT path | US1 Polish | LOW — satisfies plan §1.3 observability; not a standalone FR |
| T039 | Emit projection observability fields | US4 Polish | LOW — satisfies plan §1.3 observability; not a standalone FR |
| T041 | Bundle: macOS mic permission, ORT dylib rpath, AEC static-link, notarization | Polish | LOW — operational/release gate, out-of-scope runtime FR |
| T043 | Extend architecture tests for dependency-direction + offline-completion | Polish | LOW — cross-cutting constitution guard (Gates VI/VII); not a standalone FR |
| T044 | Update docs/architecture.md Voice pillar + AGENTS.md/CLAUDE.md lockstep | Polish | LOW — documentation maintenance |
| T045 | Performance + security pass | Polish | LOW — manual review sweep; automated component is T040a |
| T046 | Manual gate: `make gui-smoke` + real-device voice walkthrough | Polish | LOW — manual workstation gate, not CI-verifiable |

All unmapped tasks are in Setup or Polish phases. None are in story phases (US1–US4). Severity LOW across the board.

---

## Constitution Alignment

| Gate | Article | Result | Citation |
|------|---------|--------|----------|
| Test-First | I | PASS | All implementation tasks in tasks.md are preceded by a `[P]`-marked test task for the same behaviour. Detailed per-story verification in §Phase-Order Validation. |
| Evidence-Driven | II | PASS | SC-005/SC-006 carry Article II(c) rationale (whisper.cpp RTF benchmark for SC-006; usability threshold for SC-005) + headless latency-regression test T040a. FR-002 default Right-Option stated with rationale. Zero vague adjectives without quantification remain in spec.md. All challenger C6/H2 findings ACCEPTED+fixed. |
| CRITICAL-Resolved | III | PASS | challenges.md records all 6 CRITICAL + 5 HIGH + 3 MEDIUM + 1 LOW as ACCEPTED+fixed (round 1, 2026-06-21). Spec body is `[NEEDS CLARIFICATION]`-free (single match at spec.md:8 is in the authoring-rules header, not a live gap). All four Independent Tests are confirmed (no `[PROPOSED]` marker in current spec.md). |
| Independent-MVP | IV | PASS | All four P1 stories have non-empty, specific, confirmed Independent Test fields (US1: spec.md:22–23; US2: spec.md:38–39; US3: spec.md:53–54; US4: spec.md:69–70). Challenger H1 resolved: shared free-form intake routing is Foundational (T015/T016); each story adds an independent modality. Complexity Tracking entry in plan.md:145. |
| Simplicity | V | PASS (with Complexity Tracking) | Five new engines added (cpal, livekit-wakeword, voice_activity_detector, webrtc-audio-processing, ringbuf). Each has a Complexity Tracking entry in plan.md §Complexity Tracking with rejected alternatives. No new project/crate created. Gate IV override (all-P1) also has a Complexity Tracking entry. |
| Edge-Autonomy | VI | PASS | plan.md §Technical Context: "no cloud NLU, no hub round-trip"; FR-012 restates the audio-local constraint; T043 extends offline-completion architecture test; no metered API key in voice path. |
| Dependency-Direction | VII | PASS | New code lives in edge/host + edge/shell + edge/ui + shared; voice modules do not import upward. T043 extends dependency-direction.test.ts to cover new modules. Internal rule voice never depends on participants above it. |
| Event-Sourced | VIII | PASS | Voice intake emits validated Commands into dispatch; run-state changes fold through the existing pure reducer. Projection only reads the event stream. Voice enable/listening status is ephemeral UI state, not run state (plan.md §Cross-Cutting Concerns). |
| Privacy-Boundary | IX | PASS | FR-012 explicitly restates Article IX for the audio path. T018 adds a test asserting hub-sync payload transmits no audio/transcript field. Applied default D-DATA-1 cited in spec.md §Defaults Applied. |
| Schema-Validated | X | PASS | FR-013 requires schema validation at every emit/consume site. T011 tests new bus schema files; T017 tests schema-validation at every voice emit call site (FakeDispatch receives schema-valid payload; invalid payload rejected before emit). |

**All 10 gates pass. Zero constitution violations remain open.**

---

## Cross-Artifact Inconsistencies

| ID | Type | Location(s) | Details | Severity |
|----|------|-------------|---------|----------|
| X1 | Terminology: "spoken cancel" vs "spoken-cancel" | spec.md throughout, tasks.md T032/T034 | Hyphenation varies ("spoken cancel" in spec, "spoken-cancel" in tasks). Stylistic only; same concept. | LOW |
| X2 | Terminology: "always-on" vs "always on" | spec.md FR-003 "always-on", plan.md "always-on listen" | Both hyphenated consistently in both artifacts. No drift. | PASS |
| X3 | Tech consistency: `webrtc-audio-processing` | spec.md FR-011a, plan.md §Technical Context, tasks.md T004/T010, research.md R5 | All four artifacts name the same crate and `bundled` feature. | PASS |
| X4 | Path: `edge/host/schemas/bus/*.json` | plan.md §Project Structure, tasks.md T011/T012 | Both artifacts reference the same path. Directory confirmed to exist (`edge/host/schemas/bus/`). | PASS |
| X5 | FR-008 terminology | spec.md FR-008 "supervised participants of the run registry" vs tasks.md T013 "registers on a fake AgentRegistry" | Same concept; "supervised" is the spec WHAT language and "AgentRegistry" is the HOW name. Not a conflict — expected split across spec/tasks. | PASS |

**Zero cross-artifact inconsistencies of MEDIUM or higher severity.**

---

## Checklist Pass Rate

`specs/015-voice-participants/checklists/requirements.md` — 32 items total.

**Passed: 0 / 32. Pass rate: 0%.**

All 32 items are unchecked `[ ]`. The checklist has not been signed off by the engineer. Per the report rules: checklist pass rate < 100% = BLOCKED.

**Assessment of each item against spec content (for engineer reference — the validator does not check these off; the engineer does):**

Items that the spec substantively satisfies and the engineer should be able to verify and check:

| Item | Evidence in spec | Engineer action |
|------|-----------------|-----------------|
| CHK001 Primary flows covered by stories | 4 P1 stories cover PTT, hot-word, stop, projection | Engineer verify + check |
| CHK002 Every FR has ≥1 acceptance scenario | FR-001 through FR-018 each trace to a user story scenario or edge case | Engineer verify + check |
| CHK003 Data entities in Key Entities | AudioChunk, Transcript, VoiceCommand, SpeakableEvent, WakeModel listed | Engineer verify + check |
| CHK004 Dependencies with failure modes | cpal, livekit-wakeword, voice_activity_detector, webrtc-audio-processing, whisper.cpp, Kokoro, ORT, 014 registry, 013 bus each have failure modes in §Dependencies | Engineer verify + check |
| CHK005 No unquantified vague adjectives | Challenger H2 resolved; all performance targets quantified or Article II(c) rationale given | Engineer verify + check |
| CHK006 Every FR single testable statement | Inspect: FR-011 "detect AND duck/stop AND not corrupt" may be compound | Engineer verify — FR-011 may need review |
| CHK007 Terminology consistent across artifacts | No medium+ drift found | Engineer verify + check |
| CHK008 Scenarios align with FRs | US1 AS-1/2/3 align with FR-001/002/004/006/007; US2 with FR-003/004; US3 with FR-005/005a; US4 with FR-009/010/011 | Engineer verify + check |
| CHK009 No contradictory requirements | Challenger H3 (EC-002 vs FR-011a) ACCEPTED+fixed; EC-002 now states both paths | Engineer verify + check |
| CHK010 SC aligns with story value propositions | SC-001↔US1; SC-002↔US2; SC-003↔US3; SC-004↔US4; SC-005↔FR-017; SC-006↔FR-018 | Engineer verify + check |
| CHK011 Scenarios in Given/When/Then | All acceptance scenarios use G/W/T with measurable outcomes | Engineer verify + check |
| CHK012 SC technology-agnostic | SC-001 through SC-004 are behavioural; SC-005/006 name the latency targets but not the engine | Engineer verify + check |
| CHK013 Every SC verifiable without knowing implementation | SC-001–004 verifiable behaviourally; SC-005/006 measurable via T040a headless test | Engineer verify + check |
| CHK014 Happy paths covered | US1 PTT→goal, US2 wake→goal, US3 stop, US4 projection all covered | Engineer verify + check |
| CHK015 Alternate flows covered | US1 AS-2 steer (focused run); US2 AS-1 (wake over ambient) | Engineer verify + check |
| CHK016 Exception/error flows covered | EC-003 (sidecar down), EC-004 (mic denied), FR-014 | Engineer verify + check |
| CHK017 Recovery/rollback scenarios | EC-006 (toggle-off mid-capture), EC-007 (run terminates during STT) | Engineer verify + check |
| CHK018 Zero-state edge cases | EC-001 (empty utterance), US3 AS-3 (no active run on stop) | Engineer verify + check |
| CHK019 Boundary edge cases | EC-001 (silence), SC-005 (false-accept rate) | Engineer verify + check — no explicit max-utterance-length boundary |
| CHK020 Concurrency edge cases | EC-002 (barge-in), EC-007 (run terminates during STT) | Engineer verify + check |
| CHK021 Malicious-input edge cases | EC-005 (wake-word spoofing / audio injection) | Engineer verify + check |
| CHK022 Performance targets quantified | SC-006 p95 ≤1.5 s dispatch; ≤1.0 s TTS-start; always-on ≤15% core | Engineer verify + check |
| CHK023 Availability/reliability targets | Physical abort "every time" (SC-003); wake false-accept ≤1/hr (SC-005); no availability SLA stated | Engineer verify — no uptime SLA is intentional (single-operator, local) |
| CHK024 Observability in plan.md | plan.md §1.3 lists log fields, metrics, trace spans | Engineer verify + check |
| CHK025 Security in plan.md | plan.md §1.3 Security: trust boundary, AuthN, secrets, privacy | Engineer verify + check |
| CHK026 Accessibility specified | D-A11Y-1 applied; T040 extends VoiceSettingsPanel with non-color indicators + reduced-motion | Engineer verify + check |
| CHK027 Every dependency with expected behaviour + failure mode | See CHK004 — same evidence | Engineer verify + check |
| CHK028 Assumptions flagged in §Assumptions | EC-001/004/005 open markers and all ASSUMPTIONs are in §Assumptions or §Clarifications | Engineer verify + check |
| CHK029 Out-of-scope items explicit | §Out of Scope lists Windows/Linux, streaming STT, engine replacement, multi-speaker, pure-Rust VAD fallback, cloud voice | Engineer verify + check |
| CHK030 [NEEDS CLARIFICATION] resolved | Single match in spec.md is in the authoring-rules header (line 8), not a live gap | Engineer verify + check |
| CHK031 /spec clarify session run + integrated | §Clarifications §Session 2026-06-21 filled with 5 resolved questions | Engineer verify + check |
| CHK032 No undefined concepts | All referenced concepts (AgentRegistry, bus, PTT, AEC, VAD, STT, TTS, etc.) defined or cited | Engineer verify — "AEC spike" and "T004" are HOW details in spec body; check for undefined WHAT terms |

The validator cannot check these items off — that is the engineer's sign-off. The content evidence shows the spec substantively satisfies every item. The zero pass rate reflects the engineer has not yet reviewed and signed the checklist, not that the spec fails the questions.

**Items warranting closer engineer scrutiny:**

- **CHK006** — FR-011 ("MUST be detectable while TTS playing AND MUST duck/stop AND MUST NOT corrupt") is a three-part requirement. Consider whether the three parts should be split into FR-011a, FR-011b for cleaner single-statement testability.
- **CHK019** — No explicit max utterance length boundary is stated. Not a blocker but the engineer should confirm whether unbounded utterance length is intentional.
- **CHK023** — No formal uptime/availability SLA for the voice subsystem. Intentional for a single-operator local app; engineer should confirm.

---

## Phase-Order Validation

Verified: for each user story, every `[P]`-tagged test task ID precedes the implementation task that depends on it.

### Foundational (Phase 2)

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| cpal capture framing → 16 kHz mono f32 512-sample | T005 | T006 | YES — T005 before T006 |
| playback emits SpeechChunk + render-reference tap | T007 | T008 | YES — T007 before T008 |
| AEC frame adaptation (512 → 3×160 → 512) | T009 | T010 | YES — T009 before T010 (gated on T004 spike) |
| Bus schema validation (new voice shapes) | T011 | T012 | YES — T011 before T012 |
| Participant lifecycle (AgentRegistry start/stop/abort) | T013 | T014 | YES — T013 before T014 |
| Free-form routing + run-targeting + EC-007 | T015 | T016 | YES — T015 before T016 |
| Schema validation at every voice emit site | T017 | T021 | YES — T017 before T021 |
| Privacy: no audio/transcript on hub-sync | T018 | T021 | YES — T018 before T021 |
| Typed error surface (3 cases) | T019 | T021 | YES — T019 before T021 |
| Toggle gate + EC-006 mid-capture/mid-playback | T020 | T021 | YES — T020 before T021 |

### US1 — Push-to-Talk (Phase 3)

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| PTT key-down→capture→key-up→command | T022 | T023 | YES — T022 before T023 |

T024 (observability emit) is cross-cutting Polish; no preceding test task required (no state-changing impl).

### US2 — Hot-Word (Phase 4)

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| Wake detection fires; TTS echo does NOT fire (AEC) | T025 | T027 | YES — T025 before T027 |
| VAD endpointing bounds post-wake utterance | T026 | T028 | YES — T026 before T028 |
| Wake path wired to foundational intake | T025, T026 | T029 | YES — tests before T029 |

### US3 — Stop (Phase 5)

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| Physical abort → registry.cancel; no-target notification | T030 | T033 | YES — T030 before T033 |
| Integration: shortcut calls registry.cancel with no STT/NLU/LLM | T031 | T033 | YES — T031 before T033 |
| Spoken-cancel matcher — short "stop" vs "stop wasting tokens" | T032 | T034 | YES — T032 before T034 |

### US4 — Projection (Phase 6)

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| Allowlist: transmission + milestones → FakeTts; non-allowlisted → zero FakeTts | T035 | T037 | YES — T035 before T037 |
| Barge-in: wake/PTT during playback → duck/stop + capture | T036 | T038 | YES — T036 before T038 |

T039 (observability emit) is Polish with no state-changing impl; no preceding test required.

### Polish Phase

| Behaviour | Test Task | Implementation Task | Order Valid? |
|-----------|-----------|---------------------|--------------|
| VoiceSettingsPanel wake-arm toggle + indicator (D-A11Y-1) | T040 (`[P]`) | T040 (same task — UI + vitest in one) | YES — marked `[P]`, self-testing |
| Headless latency regression (SC-006) | T040a (`[P]`) | T040a (CI test only, no impl) | YES — pure test |
| Bundling, notarization | (none — manual workstation gate) | T041 | ACCEPTABLE — no automated test possible for macOS notarization; manual gate is documented |
| Wake-word model training | (none — ops step) | T042 | ACCEPTABLE — ops/manual; no runtime implementation task |
| Architecture tests extension | T043 covers this | T043 | YES |

**Article I verdict: PASS. Every behaviour-changing implementation task is preceded by a test task in tasks.md. The two manual/ops tasks (T041, T042) are correctly identified as workstation/ops gates, not automated implementation, so Article I's "production code must not be written before a failing test" does not apply to them.**

---

## Path Validity

Every file path in tasks.md is checked: either it exists in the repo now, or it is a NEW file that the task itself creates (or it is in a parent directory that exists).

| Task(s) | Path(s) | Status |
|---------|---------|--------|
| T001 | `edge/host/Cargo.toml` | EXISTS (`edge/host/` is a known directory) |
| T002 | `edge/host/tests/fixtures/voice/` | PARENT EXISTS (`edge/host/tests/fixtures/` confirmed); `voice/` subdirectory does NOT exist yet — T002 creates it. OK. |
| T003 | `edge/host/` (cargo feature gate in Cargo.toml) | EXISTS |
| T004 | `research.md` R5 (update) | EXISTS at `specs/015-voice-participants/research.md` |
| T005 | `edge/host/tests/unit/voice_capture.rs` | PARENT EXISTS (`edge/host/tests/unit/` confirmed); file is NEW, created by T005. OK. |
| T006 | `edge/host/src/voice/capture.rs` | PARENT EXISTS (`edge/host/src/voice/` confirmed); file is NEW. OK. |
| T007 | `edge/host/tests/unit/voice_playback.rs` | PARENT EXISTS; file is NEW. OK. |
| T008 | `edge/host/src/voice/playback.rs` | PARENT EXISTS; file is NEW. OK. |
| T009 | `edge/host/tests/unit/voice_aec.rs` | PARENT EXISTS; file is NEW. OK. |
| T010 | `edge/host/src/voice/aec.rs` | PARENT EXISTS; file is NEW. OK. |
| T011 | `edge/host/tests/unit/voice_schema_validate.rs` | PARENT EXISTS; file is NEW. OK. |
| T012 | `edge/host/schemas/bus/command.json` + `event.json`, `shared/contracts/*.d.ts` | `edge/host/schemas/bus/` EXISTS; `command.json` + `event.json` EXIST (confirmed). `shared/contracts/` EXISTS. OK. |
| T013 | `edge/host/tests/unit/voice_participant_lifecycle.rs` | PARENT EXISTS; file is NEW. OK. |
| T014 | `edge/host/src/participants/voice_intake.rs`, `edge/host/src/participants/voice_projection.rs` | PARENT EXISTS (`edge/host/src/participants/` confirmed). OK. |
| T015 | `edge/host/tests/unit/voice_intake_freeform.rs` | PARENT EXISTS; file is NEW. OK. |
| T016 | `edge/host/src/participants/voice_intake.rs` | PARENT EXISTS; created by T014. OK. |
| T017 | `edge/host/tests/unit/voice_emit_schema.rs` | PARENT EXISTS; file is NEW. OK. |
| T018 | `edge/host/tests/unit/voice_privacy_sync.rs` | PARENT EXISTS; file is NEW. OK. |
| T019 | `edge/host/tests/unit/voice_error_surface.rs` | PARENT EXISTS; file is NEW. OK. |
| T020 | `edge/host/tests/unit/voice_toggle_gate.rs` | PARENT EXISTS; file is NEW. OK. |
| T021 | `edge/host/src/participants/voice_intake.rs`, `voice_projection.rs`, `capture.rs`, `playback.rs` | Parents exist; files created by earlier tasks. OK. |
| T022 | `edge/host/tests/unit/voice_ptt.rs` | PARENT EXISTS; file is NEW. OK. |
| T023 | `edge/shell/src/` (PTT global shortcut wiring) | `edge/shell/src/` EXISTS. OK. |
| T025 | `edge/host/tests/unit/voice_wake.rs` | PARENT EXISTS; file is NEW. OK. |
| T026 | `edge/host/tests/unit/voice_vad.rs` | PARENT EXISTS; file is NEW. OK. |
| T027 | `edge/host/src/voice/wake.rs` | PARENT EXISTS; file is NEW. OK. |
| T028 | `edge/host/src/voice/vad.rs` | PARENT EXISTS; file is NEW. OK. |
| T029 | `edge/host/src/participants/voice_intake.rs` | Created by T014/T016. OK. |
| T030 | `edge/host/tests/unit/voice_abort_physical.rs` | PARENT EXISTS; file is NEW. OK. |
| T031 | `edge/host/tests/integration/abort_no_speech_path.rs` | `edge/host/tests/integration/` EXISTS (confirmed). file is NEW. OK. |
| T032 | `edge/host/tests/unit/voice_cancel_match.rs` | PARENT EXISTS; file is NEW. OK. |
| T033 | `edge/shell/src/` (physical abort global shortcut) | `edge/shell/src/` EXISTS. OK. |
| T034 | `edge/host/src/voice/cancel.rs` | PARENT EXISTS; file is NEW. OK. |
| T035 | `edge/host/tests/unit/voice_projection_allowlist.rs` | PARENT EXISTS; file is NEW. OK. |
| T036 | `edge/host/tests/unit/voice_barge_in.rs` | PARENT EXISTS; file is NEW. OK. |
| T037 | `edge/host/src/participants/voice_projection.rs` | Created by T014. OK. |
| T038 | `edge/host/src/participants/voice_projection.rs` + `capture.rs` | Created by earlier tasks. OK. |
| T040 | `edge/ui/app/components/VoiceSettingsPanel.tsx`, `edge/ui/tests/unit/` | `VoiceSettingsPanel.tsx` EXISTS (confirmed). `edge/ui/tests/unit/` EXISTS (confirmed). OK. |
| T040a | `edge/host/tests/unit/voice_latency_budget.rs` | PARENT EXISTS; file is NEW. OK. |
| T041 | `edge/shell/` (bundling: Info.plist, entitlements, dylib) | `edge/shell/src/` EXISTS. OK. |
| T042 | app-data (wake-word model, ops step) | No filesystem path in tasks.md — runtime app-data location; not a compile-time path. OK. |
| T043 | `tests/architecture/dependency-direction.test.ts` | EXISTS (confirmed). OK. |
| T044 | `docs/architecture.md`, AGENTS.md, CLAUDE.md | `docs/architecture.md` EXISTS (confirmed). OK. |

**Path Validity verdict: PASS. All paths either exist or are in confirmed parent directories that will contain the new files. No orphaned or dangling paths.**

---

## [NEEDS CLARIFICATION] Scan

Scanned spec.md for `[NEEDS CLARIFICATION]` marker (case-sensitive).

**Result: 1 match at spec.md:8.** This match is in the spec authoring-rules header:

> "No silent defaults — catalog defaults cited in `## Defaults Applied`, everything else `[NEEDS CLARIFICATION]`."

This is a **rule citation**, not a live gap marker. It appears as a description of the `[NEEDS CLARIFICATION]` convention, not an instance of it.

**All live `[ASSUMPTION — engineer confirm]` markers in EC-001, EC-004, EC-005 are retained intentionally** per the spec's §Assumptions section — these are documented open markers, not blocking gaps. They do not constitute `[NEEDS CLARIFICATION]` under the spec-driven workflow's definition (which covers unknown decisions, not accepted assumptions).

**[NEEDS CLARIFICATION] scan verdict: PASS. Zero live gaps.**

---

## Independent-Test Integrity

All four P1 user stories must have a non-empty, specific Independent Test (concrete action + value delivered).

| Story | Independent Test Text (from spec.md) | Specific? | Non-empty? | Status |
|-------|--------------------------------------|-----------|------------|--------|
| US1 — PTT | "With the voice toggle on and no run active, hold PTT, speak 'research the voice landscape', release; observe a new run start with that goal via the validated dispatch — verifies capture → STT → free-form command end-to-end." (spec.md:22–23) | YES | YES | PASS |
| US2 — Hot-word | "With the voice toggle on, say 'Hey Wagner, summarize my open runs' with no key held; observe wake detection, utterance endpointing, and the same goal submission as the PTT path — verifies wake-word + VAD + capture → STT → command." (spec.md:38–39) | YES | YES | PASS |
| US3 — Stop | "With a run active and targeted, trigger the physical abort control; observe the run cancel via the run-supervision cancel path with no speech recognition involved — verifies the deterministic stop. (Spoken cancel verified separately as best-effort.)" (spec.md:53–54) | YES | YES | PASS |
| US4 — Projection | "With the voice toggle on, trigger an agent transmission and then a run completion; observe both spoken (transmission text, then the 'run complete' canned phrase) while a non-allowlisted activity event stays silent — verifies the allowlist projection." (spec.md:69–70) | YES | YES | PASS |

**No `[PROPOSED]` markers remain. All four pass. Independent-Test Integrity: PASS.**

---

## Open Blockers

The sole blocker is the checklist pass rate.

**Blocker 1 (BLOCKED): Checklist pass rate = 0% (0/32 items checked).**

`specs/015-voice-participants/checklists/requirements.md` has 32 items, none checked. Per the validator's verdict rules: checklist pass rate < 100% = BLOCKED.

**What the engineer must do:** Review each CHK item against the spec, verify the evidence, and mark `[x]` for each item that passes. The spec content substantively satisfies every item (see §Checklist Pass Rate above). This is a sign-off gate, not a content rewrite.

**Note for CHK006 (compound FR):** FR-011 ("MUST be detectable while TTS playing AND duck/stop AND not corrupt either stream") contains three obligations. The engineer should decide whether to split it or accept it as one testable behaviour with three verifiable conditions. This is a judgment call, not a finding.

---

## Re-run Determinism Note

This report is deterministic. A re-run against the same artifacts (spec.md, plan.md, tasks.md, checklists/requirements.md, constitution.md, defaults.md, research.md, challenges.md) with no changes will produce:

- Verdict: BLOCKED
- Coverage: 26/26 (100%)
- Checklist: 0/32 (0%)
- Constitution gates: 10/10 PASS
- CRITICAL findings: 0 (open)
- Blockers: 1 (checklist not signed off)

If a re-run produces different findings, the artifacts changed between runs.
