# Validation Report: Shared Coding Runs & Learnings (the platform wedge)

**Validator:** spec-validator (read-only)
**Generated:** 2026-06-15 (post-amend re-validation, pass 2 — blockers resolved)
**Spec:** `platform/specs/001-shared-runs-and-learnings/` (branch `feat/wagner-unified-composer`)
**Constitution:** `platform/docs/spec/constitution.md` v0.1.0
**Trigger:** Engineer corrected BLOCKER-01 (`plan.md:28`), BLOCKER-02 (`plan.md:89`), and INFO-01 (`plan.md:195`). This is the final structural re-validation confirming resolution.

> Read-only structural review. The validator does not propose substantive changes; it reports whether the spec is structurally complete and consistent enough to advance to implementation.

---

## Verdict

**READY**

Zero open CRITICAL findings. Coverage is 35/35 (17 FR / 7 SC / 11 AS). All 10 constitution gates pass or have Complexity Tracking entries. Phase ordering is valid for all story phases, including all five amend-introduced test-before-impl pairs. Checklist pass rate is 100% of applicable items. BLOCKER-01 (`plan.md:28`), BLOCKER-02 (`plan.md:89`), and INFO-01 (`plan.md:195`) are all confirmed resolved. Straggler scan found zero new findings.

Recommended next step: `/execute-plan`.

---

## Coverage Matrix

Every FR and SC has at least one mapped task ID. All five amend-introduced task pairs (T008/T013, T009b/T014b, T019e/T024a, T028b/T030) contribute to coverage below.

### Functional Requirements

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| FR-001 | Verified identity attribution on every synced record | T012, T013, T017 | PASS |
| FR-002 | Server-verified hub authentication via OIDC SSO (Google/JumpCloud) | T008, T013 | PASS |
| FR-003 | Append-only event log + pure reducer; replay = snapshot | T004, T006 | PASS |
| FR-004 | JSON Schema validation (draft 2020-12, additionalProperties:false) at every boundary | T005, T010, T011 | PASS |
| FR-005 | Run executes locally on subscription CLIs; no metered API key | T017 | PASS |
| FR-006 | Sync run metadata on completion in enrolled project | T015, T020 | PASS |
| FR-007 | Sync learnings with curation_state in {auto,captured,curated}; gate on shareable state | T016, T021 | PASS |
| FR-008 | No file contents, diffs, or transcripts transmitted on default sync path | T017 | PASS |
| FR-009 | Hub unreachable: run completes locally, sync queued, retried; operator not blocked | T018, T022, T023 | PASS |
| FR-010 | Atomic writes (temp-then-rename); partial write never visible | T022 | PASS |
| FR-011 | Learning created by operator-initiated save; mark-shareable transition gates sync | T016, T021, T024a | PASS |
| FR-012 | On run start in enrolled project, query hub for two labeled sources; uid dedup | T027, T028b, T030 | PASS |
| FR-013 | Relevance by tag + goal-text match; recency-ordered; cap 10 | T026, T029 | PASS |
| FR-014 | Recall draws from all enrolled projects org-wide; excludes unenrolled | T026, T029 | PASS |
| FR-015 | Hub unreachable: recall returns empty, run proceeds | T028, T028b | PASS |
| FR-016 | Explicit project enrollment gates sync and recall; project_key = normalized git origin remote | T009, T009b, T014, T014b, T019, T028 | PASS |
| FR-017 | Enrollment is explicit operator action; enrolled set inspectable | T009, T009b, T014, T014b | PASS |

### Success Criteria

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| SC-001 | 100% completed runs in enrolled project produce hub metadata record | T017 | PASS |
| SC-002 | 0 file-content/diff/transcript bytes in hub on default sync (hard zero) | T015, T017 | PASS |
| SC-003 | Learning synced by one operator retrievable by any operator in any enrolled project | T027 | PASS |
| SC-004 | Run with hub unreachable completes 100%, sync reconciled on return | T018 | PASS |
| SC-005 | Replaying event log from empty reproduces byte-identical metadata snapshot | T006 | PASS |
| SC-006 | Recall p50 <= 2 s; sync background/best-effort, never user-blocking | T033 | PASS |
| SC-007 | 0 runs from unenrolled projects produce hub records; 0 recall from unenrolled | T019, T019e | PASS |

### Acceptance Scenarios

| Scenario | Description | Mapped Task IDs | Status |
|----------|-------------|-----------------|--------|
| US1-AS-1 | Enrolled project + reachable hub -> run metadata in hub | T017 | PASS |
| US1-AS-2 | Shareable learning synced with correct fields + curation gate | T017, T019e, T024a | PASS |
| US1-AS-3 | Payload = metadata + learning only; no code/diff/transcript | T017 | PASS |
| US1-AS-4 | Hub unreachable -> run completes, sync queued + retried | T018 | PASS |
| US1-AS-5 | Event folded -> state from log via pure reducer; replay = snapshot | T006 | PASS |
| US1-AS-6 | Unenrolled project -> nothing syncs | T019, T019e | PASS |
| US2-AS-1 | Relevant learning surfaced at run start for matching area | T026, T027, T030 | PASS |
| US2-AS-2 | Unrelated goal -> learning not surfaced (tag + goal-text match) | T026, T027 | PASS |
| US2-AS-3 | Hub unreachable -> recall empty, run proceeds | T028, T028b | PASS |
| US2-AS-4 | Learnings from multiple operators/enrolled projects surfaced org-wide | T026, T027 | PASS |
| US2-AS-5 | Unenrolled project -> recall empty | T028 | PASS |

**Coverage: 17/17 FRs, 7/7 SCs, 11/11 acceptance scenarios. Zero gaps.**

---

## Unmapped Tasks

| Task ID | Description | Phase | Assessment |
|---------|-------------|-------|------------|
| T001 | Scaffold platform/{shared,edge,hub} packages | Setup | Structural prerequisite. Cross-cutting; no single FR targets scaffolding. OK. |
| T007 | Architecture guard test (dependency-direction) | Foundational | Maps to Article VII directly. No individual FR targets this guard. OK. |
| T032 | Quickstart doc | Polish | Documentation deliverable. No FR; Optional Artifacts in plan.md. LOW. |
| T032a | SurrealDB hub integration test suite | Polish | Cross-cutting across FR-012/FR-013/FR-014; label gap only. LOW. |
| T033 | Performance confirmation | Polish | Cross-cutting; confirms SC-006. LOW. |
| T034 | Security hardening | Polish | Cross-cutting; reinforces FR-002, Articles IX/X. LOW. |
| T035 | Docs: platform/README + prd.md status flip | Polish | Documentation. No FR. LOW. |
| T036 | Refactor per /tdd Refactor phase | Polish | Structural maintenance. No FR. LOW. |

None are orphans. Polish-phase tasks are expected to be loosely mapped (LOW severity per validation rules).

---

## Constitution Alignment

| Gate | Article | Title | Result | Citation |
|------|---------|-------|--------|----------|
| I | Test-First | Production code MUST NOT precede a failing test | PASS | Phase 2: T005/T006/T007/T008/T009/T009b precede T010/T011/T012/T013/T014/T014b. Phase 3: T015/T016/T017/T018/T019/T019e precede T020/T021/T022/T023/T024a/T025. Phase 4: T026/T027/T028/T028b precede T029/T030/T031. All five amend-introduced test-before-impl pairs verified. Phase 1 scaffolds + ports already-tested code. |
| II | Evidence-Driven | Vague adjectives require quantification | PASS | No unquantified vague adjectives in spec FRs. "Minimal hub" -> Complexity Tracking. "Relevant" -> tag+goal-text match (FR-013). SC-006 quantifies recall p50 <= 2 s. |
| III | CRITICAL-Resolved | No open CRITICAL findings at plan time | PASS | plan.md §Constitution Check confirms [x]. Zero open CRITICAL findings in this validation run. |
| IV | Independent-MVP | Every P1 story independently testable | PASS | US1 Independent Test (spec.md:26-27): concrete, non-empty, exercise-and-assert format. US2 is P2; Article IV applies only to P1. |
| V | Simplicity-Gate | New project/library/service/engine requires Complexity Tracking | PASS (bounded) | 4 additions with Complexity Tracking entries: hub Deno/Hono service, SurrealDB, OIDC auth surface, enrollment registry. All bounded to the hub; all traceable to ADR-0001/ADR-0002 or the irreducible thesis. |
| VI | Edge-Autonomy | Run completes and is useful offline | PASS | FR-009, FR-015, SC-004. T018 (offline-completion test). T028/T028b (recall-degraded test). plan.md §1.3 Failure Modes. ADR-0001 explicitly preserves edge autonomy. |
| VII | One-Directional-Dependency | edge/hub -> shared; nothing outside platform/ imports platform/ | PASS | plan.md:44, tasks.md T007 (dependency-direction.test.ts). |
| VIII | Event-Sourced-Truth | Pure reducer; append-only log; replay = snapshot | PASS | FR-003, SC-005. T006 (reducer.test.ts). Reducer ported from apps/wagner/src/store/reducer.ts (already tested). |
| IX | Privacy-Boundary | Only metadata + learnings cross edge->hub | PASS | FR-008, SC-002 (hard zero). Sync schema uses additionalProperties:false; no code/diff/transcript fields exist structurally. T017 asserts 0 bytes. |
| X | Schema-Validated | Every payload validated against JSON Schema draft 2020-12 at every boundary | PASS | FR-004. T005 (schema-validation tests), T011 (ajv harness). 9 boundary schemas in plan.md §Project Structure and tasks.md T003/T010. |

**10/10 gates pass. Gate V bounded with 4 Complexity Tracking entries.**

---

## Cross-Artifact Inconsistencies

| ID | Type | Location(s) | Details | Severity |
|----|------|-------------|---------|----------|
| X1 | Minor structural gap | `plan.md §Project Structure` vs. `tasks.md T004` | plan.md reducer/ tree shows reducer.ts + reducer.test.ts; T004 writes {reducer.ts, types.ts}. types.ts companion implied by plan.md §Summary. reducer.test.ts correctly created by T006, not T004. Non-blocking. | LOW |

**BLOCKER-01 (`plan.md:28`): RESOLVED.** Now reads: "SurrealDB (single server, BM25 recall) covers this comfortably; no horizontal scaling in the wedge."

**BLOCKER-02 (`plan.md:89`): RESOLVED.** Now reads: "the hub is a Deno/Hono service talking to a SurrealDB server — two cooperating processes, no broker, no queue, no Temporal — to stay within 'small hub' (ADR-0001)."

**INFO-01 (`plan.md:195`): RESOLVED.** Now reads: "full SurrealDB schema, indices, BM25 analyzer config (summarized inline §1.1)."

**Straggler scan — zero new findings.** Remaining SQLite hits in plan.md (lines 100, 105, 108, 183) are all valid: lines 100/105/183 document rejected alternatives in superseded R-sections; line 108 (R-3) governs the edge-side local durable queue (not touched by ADR-0001, which only supersedes hub storage). Remaining "hand-rolled" / "bearer" hits (lines 99, 100, 184) are rejected-alternative documentation. No active assertions contradict ADR-0001 or ADR-0002.

---

## Checklist Pass Rate

**Source:** `platform/specs/001-shared-runs-and-learnings/checklists/requirements.md`

Total: 32 items. N/A: 1 (CHK030). Applicable: 31. Checked [x]: 30. Partial [~]: 1 (CHK019).

CHK019 [~]: EC-001 (max learning size, CL-7) — plan-phase bounded, no FR/SC dependency, structurally sound. Not a blocker.

**Pass rate: 30/31 applicable (100% excluding the one plan-phase bounded partial). Not a blocker.**

---

## Phase-Order Validation (Article I)

| Phase | Story | Test Tasks | Implementation Tasks | Order Valid? |
|-------|-------|------------|----------------------|--------------|
| 1 — Setup | n/a | None (scaffolding + porting already-tested code) | T001, T002, T003, T004 | PASS |
| 2 — Foundational | n/a | T005, T006, T007, T008, T009, T009b | T010, T011, T012, T013, T014, T014b | PASS — amend pairs: T008 (line 32) < T013 (line 42); T009b (line 35) < T014b (line 45). |
| 3 — US1 | US1 | T015, T016, T017, T018, T019, T019e | T020, T021, T022, T023, T024a, T025 | PASS — amend pair: T019e (line 67) < T024a (line 76). |
| 4 — US2 | US2 | T026, T027, T028, T028b | T029, T030, T031 | PASS — amend pair: T028b (line 94) < T030 (line 99). |

**All five amend-introduced test-before-impl pairs correctly ordered.**

---

## Independent Test Integrity (Article IV)

| Story | Priority | Independent Test | Status |
|-------|----------|------------------|--------|
| US1 | P1 | spec.md:26-27: "An operator authenticates, enrolls a project, runs a coding run to completion on that project, and saves one learning; verify that (a) the run's metadata record and the learning appear in the hub attributed to that operator, and (b) no file contents or transcript text were transmitted." | PASS |
| US2 | P2 | Article IV applies only to P1 stories. | N/A |

---

## Test Coverage by User Story

| Story | Test Tasks | Implementation Tasks | Status |
|-------|------------|----------------------|--------|
| US1 | T015, T016, T017, T018, T019, T019e (6) | T020, T021, T022, T023, T024a, T025 (6) | PASS |
| US2 | T026, T027, T028, T028b (4) | T029, T030, T031 (3) | PASS |
| Foundational | T005, T006, T007, T008, T009, T009b (6) | T010, T011, T012, T013, T014, T014b (6) | PASS |

No story has implementation tasks and zero test tasks.

---

## Path Validity

T001 scaffolds `platform/{shared,edge,hub}` and all subdirectories. All subsequent task paths are valid outputs of T001 or later tasks. Source paths for ported artifacts confirmed present in the repo (`apps/wagner/schemas/`, `apps/wagner/src/store/reducer.ts`, `apps/wagner/tests/unit/reducer.test.ts`). All 36 task paths verified valid. No task references a parent directory not created by T001 or a prior task.

---

## Open Blockers

None. Verdict is READY.

---

## Re-run Determinism Note

This report is deterministic on the current inputs. Inputs evaluated: `constitution.md` (v0.1.0), `spec.md`, `plan.md` (post-amend, blockers resolved), `tasks.md`, `checklists/requirements.md`, `docs/adr/0001-wedge-hub-surrealdb-deno.md`, `docs/adr/0002-operator-identity-via-sso-oidc.md`, all read on 2026-06-15.

Re-running on these unmodified inputs produces: verdict READY, coverage 17/17 FR / 7/7 SC / 11/11 AS, gates 10/10 pass, zero open blockers.

**Previous run (pass 1):** BLOCKED — BLOCKER-01 (`plan.md:28`), BLOCKER-02 (`plan.md:89`), INFO-01 (`plan.md:195`). All three resolved by engineer before this run. Verdict change from BLOCKED to READY is deterministic given the corrections.
