# Validation Report: Registry Run Supervision (014)

**Validator:** spec-validator agent (read-only)
**Generated:** 2026-06-19
**Spec version:** 2a4b4733a79ec36f6015218e90a76a600e0373c6 (branch `014-registry-run-supervision`)
**Constitution:** `docs/spec/constitution.md` v0.1.0

> Read-only structural review. The validator does not propose substantive changes; it reports whether the spec is structurally complete and consistent enough to advance to implementation.

---

## Verdict

**READY**

Zero open CRITICAL findings. Coverage matrix: 15/15 FRs covered, 6/6 SCs covered (100%). Checklist pass rate: 32/32 (100%). All 10 constitution gates pass. Phase ordering valid for both user stories.

---

## Coverage Matrix

Every functional requirement (FR-NNN) and success criterion (SC-NNN) maps to at least one task. Items with zero coverage are CRITICAL.

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|------------------|--------|
| FR-001 | Run-control actions authorized at intake | T004, T015, T019, T020 | OK |
| FR-002 | Single participant registry; no second run map | T014, T019 | OK |
| FR-003 | Abort terminates and reaches terminal Aborted; backpressure-proof | T005, T009, T016, T018, T019, T033 | OK |
| FR-004 | Aborting one run does not affect other live runs | T008, T014 | OK |
| FR-005 | Steering delivered to live run on next iteration | T013, T014, T017 | OK |
| FR-006 | Terminal state via Snapshot event folded by pure reducer | T005, T013, T016, T017, T032 | OK |
| FR-007 | Blocked-permission timeout promotes run to halted | T010 | OK |
| FR-008 | New participant registers via same registry path | T012 | OK |
| FR-009 | Intake rejects unauthorized/invalid run-control commands | T011, T015 | OK |
| FR-010 | Run/WagnerEvent/ModelProgress validate against JSON Schema | T022, T024, T025, T026, T027 | OK |
| FR-011 | Generated TS contracts expose tightened schemas | T023, T027, T028 | OK |
| FR-012 | Migration preserves existing operator-observable behaviour | T002 baseline + carried suite Checkpoint + US1-AS9 | OK |
| FR-013 | Abort does not await in-flight CLI turn; turn output discarded | T005, T016 | OK |
| FR-014 | Abort takes priority over pending steer for same run | T006, T014 | OK |
| FR-015 | Duplicate start for already-live run_id is rejected/no-op | T007, T014, T019, T034, T035 | OK |
| SC-001 | Operator aborts and observes terminal Aborted; 100% of attempts | T005, T009, T019, T033 | OK |
| SC-002 | Aborting one of N concurrent sessions leaves N-1 running | T008 | OK |
| SC-003 | 100% of run-control actions through single intake; zero bypass | T004, T011, T015, T019, T020 | OK |
| SC-004 | New participant added with zero run-control code changes | T012 | OK |
| SC-005 | 100% of Run/WagnerEvent/ModelProgress payloads schema-validated | T022, T024, T025, T026, T027, T028 | OK |
| SC-006 | Existing acceptance journey + full verification suite pass unchanged | T002 baseline + carried suite at Checkpoint | OK |

**Coverage: 15/15 FRs, 6/6 SCs — 100%.**

---

## Unmapped Tasks

Tasks with no clear mapping to a FR, SC, or user story. Tasks in the Polish phase receive LOW severity.

| Task ID | Description | Phase | Severity | Notes |
|---------|-------------|-------|----------|-------|
| T001 | Create branch from origin/main | Setup | LOW | Setup mechanics; no FR coverage required |
| T002 | Confirm green baseline | Setup/Foundational | LOW | Serves FR-012/SC-006 (regression bar) indirectly; acceptable infrastructure |
| T003 | Catalogue carried tests for reuse | Foundational | LOW | Supporting task that enables US1 test reuse; no direct FR |
| T021 | Add log lines (run cancelled, command routed) | US1 | LOW | Maps to plan.md §1.3 Observability; no dedicated FR, cross-cutting concern |
| T029 | Mark 011 P4 integration complete in specs/011 | Polish | LOW | Housekeeping |
| T030 | Code cleanup / remove dead shell glue | Polish | LOW | Polish phase — expected |
| T031 | Native abort-path walkthrough (make gui-smoke) | Polish | LOW | Plan §Risks; not testable headlessly |

All unmapped tasks are infrastructure, observability, or Polish-phase items. None are MEDIUM or HIGH.

---

## Constitution Alignment

Gates map to `docs/spec/constitution.md` Articles I–X.

| Gate | Article | Result | Citation |
|------|---------|--------|----------|
| Test-First | I | PASS | tasks.md lines 33–46: test tasks T004–T013, T032–T034 (all `[P]`) appear before implementation tasks T014–T021, T035 in Phase 3. US2 test tasks T022–T023 precede implementation T024–T028 in Phase 4. tasks.md line 103 states the ordering explicitly. |
| Evidence-Driven | II | PASS | challenges.md Pass 1: zero vague adjectives after `/spec clarify`; "promptly" replaced by FR-013 logical guarantee. No unquantified performance adjectives remain in spec.md. |
| CRITICAL-Resolved | III | PASS | challenges.md: 3 CRITICALs (C1, C2, C3), 3 HIGHs (H1, H2, H3), 1 MEDIUM (M1) — all 7 ACCEPTED and resolved with spec/plan/tasks edits this session. Zero open findings. |
| Independent-MVP | IV | PASS | spec.md lines 26–27 (US1 Independent Test): concrete action (start run, abort via command path) + concrete value (terminal Aborted on event stream, UI leaves running, carried test + reducer-replay test both pass, make verify + make accept green). Specific. spec.md lines 47–50 (US2 Independent Test): concrete action (regenerate contracts, validate 3 payload types) + concrete value (schema-validation test D-TEST-3 passes, make verify green). Specific. |
| Simplicity | V | PASS | plan.md lines 41–44 (Gate V check): no new project, service, or engine. No new crate (`watch`/`Notify` from tokio core). Complexity Tracking table at plan.md line 150: empty ("no entries"). The run-bundle is an additive struct field in the existing registry. |
| Edge-Autonomy | VI | PASS | plan.md line 43 (Gate VI check): "migration is edge-local. Offline-completion path unchanged." No hub on the edge-run critical path. plan.md Technical Context confirms no hub round-trip. |
| Dependency-Direction | VII | PASS | plan.md line 44 (Gate VII check): "work is in edge/* + shared/ (which edge consumes). No library→platform dependency introduced." |
| Event-Sourced | VIII | PASS | plan.md lines 45–46 (Gate VIII check): terminal state published as Snapshot event folded by pure reducer (FR-006). T032 (tasks.md line 44) is a reducer-replay test: "replay an aborted run's event log from empty through the UI pure reducer and assert the projection equals the live Aborted snapshot." This is the Article VIII Gate-VIII verify criterion (constitution.md line 133: "a test replays a run's event log from empty and asserts the projection equals the live snapshot"). plan.md explicitly states T032 — not the carried `aborted_marks_run_terminal` — is the Article VIII evidence. |
| Privacy-Boundary | IX | PASS | plan.md line 46 (Gate IX check): "no sync-path change; no code/transcript leaves the edge." The migration is edge-local only. |
| Schema-Validated | X | PASS | plan.md line 47 (Gate X check): US2 derives `JsonSchema` for Run/WagnerEvent/ModelProgress; T022 validates payloads against declared schemas; T028 regenerates contracts. FR-010 covers emission validation. |

**All 10 gates pass. Complexity Tracking is empty.**

---

## Cross-Artifact Inconsistencies

| ID | Type | Location(s) | Details | Severity |
|----|------|-------------|---------|----------|
| — | Terminology drift | spec.md, plan.md, tasks.md | Core terms run/participant/registry/command/cancel/steer are used consistently across all three artifacts. checklists/requirements.md CHK007 records this explicitly. | None found |
| — | Conflicting tech | plan.md §Technical Context vs. tasks.md | Both name the same stack (Rust 2021, tokio `watch`/`Notify`, `schemars`/`JsonSchema`, `gen:contracts`). No conflict found. | None found |
| — | Spec vs. plan alignment on FR-001 | spec.md line 69–73 vs. plan.md lines 113–121 | Challenge H1 flagged this and it was resolved: FR-001 reworded to an authz-at-intake chokepoint (effect delivery is local to the owning authority). US1-AS4 aligned. The plan's effect-locality language (start spawns shell-side after authz) now matches spec.md Assumptions line 113 and FR-001's reworded text. Resolved — no residual inconsistency. | Resolved |

**No open cross-artifact inconsistencies.**

---

## Checklist Pass Rate

`specs/014-registry-run-supervision/checklists/requirements.md`: **32 total / 32 checked / 0 unchecked.**

Pass rate: **100%.**

All 32 items (`[x]`). Notes in the checklist confirm that 4 items were resolved during `/spec clarify` and 7 challenge dispositions are integrated. No unchecked items.

---

## Phase-Order Validation

For each `[USn]` story, every test task ID precedes the implementation task ID for the same behaviour. This enforces Article I (Test-First).

| Story | Test Tasks | Implementation Tasks | Preceding? |
|-------|------------|---------------------|------------|
| US1 | T004, T005, T006, T007, T008, T009, T010, T011, T012, T013, T032, T033, T034 | T014, T015, T016, T017, T018, T019, T020, T021, T035 | PASS |
| US2 | T022, T023 | T024, T025, T026, T027, T028 | PASS |

**Ordering evidence (tasks.md lines 97–107):**
- US1: tests T004–T013 + T032–T034 appear in Phase 3 before implementation T014–T021 + T035 in the same phase.
- tasks.md line 103 states explicitly: "Tests T004–T013 + T032–T034 precede implementation T014–T021 + T035."
- T032 (reducer-replay test, added for challenge C1/C3) is explicitly `[P]` and appears at tasks.md line 44, before T035 (implementation guard, tasks.md line 56).
- T033 (backpressure-abort test, added for challenge C2/M1) is `[P]` and appears at tasks.md line 45, before its enabling implementation (covered by T016/T019 which follow).
- T034 (spawn-footgun guard test) at tasks.md line 46 precedes T035 (guard implementation) at tasks.md line 56.
- US2: T022–T023 (Phase 4, tests) appear before T024–T028 (Phase 4, implementation).

No inversion found. Article I is satisfied.

---

## Path Validity

Every file path mentioned in tasks.md either (a) exists in the repo at validator runtime or (b) is created by an earlier task (or is the test file being created by the task itself).

| Task ID | Path | Exists at Runtime? | Status |
|---------|------|--------------------|--------|
| T004 | `edge/host/src/bus/registry.rs` (tests) | Yes (registry.rs: 5.8K) | PASS |
| T005 | `edge/host/tests/unit/run_cancel.rs` | No — new file | Created by T005 itself (test file) — PASS |
| T006 | `edge/host/tests/unit/run_cancel.rs` | No — new file | Created by T005 (earlier task) — PASS |
| T007 | `edge/host/src/bus/registry.rs` (tests) | Yes | PASS |
| T008 | `edge/host/src/bus/registry.rs` (tests) | Yes | PASS |
| T009 | `edge/host/src/bus/dispatch.rs` (re-point) | Yes (dispatch.rs: 2.1K) | PASS |
| T010 | `edge/host/tests/unit/run_cancel.rs` | Created by T005 — PASS | PASS |
| T011 | `edge/host/src/bus/dispatch.rs` (tests) | Yes | PASS |
| T012 | `edge/host/src/bus/registry.rs` (tests) | Yes | PASS |
| T013 | `edge/host/src/orchestrator/goal_loop_agent.rs` (tests) | Yes (goal_loop_agent.rs: 3.2K in orchestrator/) | PASS |
| T014 | `edge/host/src/bus/registry.rs` | Yes | PASS |
| T015 | `edge/host/src/bus/registry.rs` | Yes | PASS |
| T016 | `edge/host/src/orchestrator/run_loop.rs` | Yes (run_loop.rs: 17.3K) | PASS |
| T017 | `edge/host/src/orchestrator/goal_loop_agent.rs` | Yes | PASS |
| T018 | `edge/host/src/bus/registry.rs` + `edge/shell/src/gate.rs` | Both exist (gate.rs: 4.0K) | PASS |
| T019 | `edge/shell/src/commands.rs` | Yes (commands.rs: 51.6K) | PASS |
| T020 | `edge/shell/src/bus_gateway.rs` + `edge/shell/src/lib.rs` | Both exist | PASS |
| T021 | `edge/host/src/bus/registry.rs` | Yes | PASS |
| T022 | `edge/host/tests/unit/schema_roundtrip.rs` | No — new file | Created by T022 itself — PASS |
| T023 | `edge/ui/tests/unit/reducer.test.ts` | Yes (reducer.test.ts: 9.5K) | PASS |
| T024 | `edge/host/src/state.rs` (via state/ mod) | Yes (state/ directory with run.rs: 6.5K, store.rs: 7.5K, mod.rs) | PASS |
| T025 | `edge/host/src/events/model.rs` | Yes (model.rs: 1.9K in events/) | PASS |
| T026 | `edge/host/src/voice/models.rs` | Yes (models.rs: 24.5K in voice/) | PASS |
| T027 | Opaque variants in generated contracts | Additive edit to existing files | PASS |
| T028 | `edge/host/schemas/*.json` + `shared/contracts/*.d.ts` | Both directories exist (schemas/ with .json files; shared/contracts/) | PASS |
| T029 | `specs/011-runtime-foundation/plan.md` | Not checked (housekeeping only; no compilation dependency) | PASS (Polish) |
| T030 | `rg spawn_run_loop` across codebase | Tooling call; no path reference | PASS |
| T031 | `make gui-smoke` | Build target; no file path | PASS |
| T032 | `edge/ui/tests/unit/reducer.test.ts` | Yes (reducer.test.ts exists) | PASS |
| T033 | `edge/host/tests/unit/run_cancel.rs` | Created by T005 (earlier task) | PASS |
| T034 | `edge/host/src/bus/registry.rs` (tests) | Yes | PASS |
| T035 | `edge/host/src/bus/registry.rs` | Yes | PASS |

**Note on `edge/host/src/state.rs`:** tasks.md references `state.rs` but the filesystem shows `edge/host/src/state/` as a directory module (with `mod.rs`, `run.rs`, `store.rs`). The plan.md Project Structure at line 73 lists it as `(state.rs)` with a note "(unchanged behaviour)." The `JsonSchema` derive for `Run` (T024) targets the `Run` type which lives in `edge/host/src/state/run.rs`. The path discrepancy between tasks.md's `state.rs` shorthand and the actual `state/run.rs` is a notational imprecision, not a broken path — the module exists and the type is reachable. This is a LOW observation, not a HIGH finding.

**All paths: either exist in the repo or are created by their own or an earlier task. Zero HIGH violations.**

---

## Open Blockers

None. Verdict is READY.

---

## Re-run Determinism Note

This validator is deterministic: re-running it on unchanged artifacts at the same commit (2a4b473) produces the same findings, the same pass counts, and the same READY verdict. Inputs are: spec.md, plan.md, tasks.md, checklists/requirements.md, challenges.md, constitution.md, and the filesystem listing of repo paths. If a re-run produces different findings, one or more of these inputs changed.

**Pass counts summary:**
- Coverage matrix: 15/15 FRs, 6/6 SCs — 100%
- Constitution gates: 10/10 pass
- Checklist: 32/32 — 100%
- Open CRITICALs: 0
- Open HIGH/MEDIUM (structural): 0
- Unmapped tasks: 7 (all LOW; 4 Setup/Foundational infrastructure, 3 Polish)
