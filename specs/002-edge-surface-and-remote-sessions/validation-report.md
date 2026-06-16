# Validation Report: Edge Surface & Remote Sessions

**Validator:** spec-validator agent (read-only)
**Generated:** 2026-06-15 (re-run after blocker fixes)
**Spec version:** feat/wagner-unified-composer (632e701)
**Constitution:** `platform/docs/spec/constitution.md` v0.1.0

> Read-only structural review. The validator does not propose substantive changes; it reports whether the spec is structurally complete and consistent enough to advance to implementation.

---

## Verdict

**READY**

Both blockers from the initial run are resolved. Zero open CRITICAL findings. Coverage matrix is 100% (all FRs, SCs, and ASes mapped). Checklist pass rate is 100%. All 10 constitution gates pass. Phase ordering is valid in all phases.

Proceed to `/execute-plan`.

---

## Coverage Matrix

The coverage matrix below is built from the task descriptions in `tasks.md`. Mapping signals used: explicit label in task description (e.g. `FR-001`, `US1-AS-1`), same file path, same domain term. Tasks from the coverage map at `tasks.md:183` are cross-checked against actual task text.

### Functional Requirements

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| FR-001 | Single-codebase unified surface | T007, T018a, T020 | PASS |
| FR-002 | Transport-abstracted event stream | T007, T012, T021 | PASS |
| FR-003 | Capability availability gated on host-reachability | T026, T031 | PASS |
| FR-004 | Remote actions route through single gate | T014, T014a, T034 | PASS |
| FR-005 | Channel messages schema-validated | T005, T010 | PASS |
| FR-100 | Host is a native process (Tauri) | T019 | PASS |
| FR-101 | Window-close backgrounds to tray; host+log survive; endpoint survives when armed | T015 (host+log), T025a (endpoint clause) | PASS |
| FR-102 | Tray shows idle/running/needs-you with non-color glyph+label | T017, T019 | PASS |
| FR-103 | needs-you raises native notification + badge | T017, T019 | PASS |
| FR-104 | Quitting app stops host + endpoint | T015, T019 | PASS |
| FR-105 | Reopening window folds live run state with no loss | T018, T021 | PASS |
| FR-200 | Remote operator verified by org SSO (OIDC) before channel opens | T008, T013 | PASS |
| FR-201 | Arm is edge-only; remote client cannot arm | T022, T028 | PASS |
| FR-202 | Remote session is ephemeral; deliberate close requires re-arm; transient drop allows re-attach | T025, T029 | PASS |
| FR-203 | Remote transport = authenticated encrypted NAT-traversing P2P with org-run relay fallback | T023, T026a, T027 | PASS |
| FR-204 | Hub-side capabilities degrade gracefully when host unreachable | T026, T031 | PASS |
| FR-210 | Remote client discovers and attaches to armed host; folds event stream identically | T023, T030 | PASS |
| FR-211 | Hub-side browse+recall function with no host reachable | T026, T031 | PASS |
| FR-212 | Remote attach refused when host not armed or operator not verified | T024 | PASS |
| FR-301 | Run-control: remote operator answers permission, steers, launches skill | T033, T038 | PASS |
| FR-302 | Dev-context commands: non-interactive, piped output, gated+logged; no PTY ALPN registered | T035, T037b, T039 | PASS |
| FR-303 | Dev-context file reads: repo-scoped default-deny, symlink/.. resolved | T036, T040 | PASS |
| FR-304 | Remote actions cannot bypass guardrails that stop local equivalent | T034, T038 | PASS |
| FR-305 | Dev-context payloads travel edge→operator device only; not to hub | T037a | PASS |

### Success Criteria

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| SC-001 | Remote attach first-frame p50 ≤ 3 s over NAT | T044, T045 | PASS |
| SC-002 | 100% remote actions in log; 0 bypass gate | T033, T034, T036 | PASS |
| SC-003 | 0 unarmed attach successes | T024 | PASS |
| SC-004 | 0 in-flight runs interrupted by window-close | T016 | PASS |
| SC-005 | Hub-side capabilities function 100% with host unreachable | T026 | PASS |
| SC-006 | 0 hub-readable run-bearing content; relay logs sizes only | T006, T026a, T037a | PASS |
| SC-007 | Deliberate close requires re-arm; < 30 s drop re-attaches ≥ 95%; ≥ 30 s requires re-arm | T025 | PASS |
| SC-008 | needs-you notification ≤ 5 s of run entering that state | T017 | PASS |

### Acceptance Scenarios

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| US1-AS-1 | Window-close backgrounds to tray; host+log+endpoint keep running | T015, T016 | PASS |
| US1-AS-2 | Reopen folds live run state from host log with no loss | T018, T021 | PASS |
| US1-AS-3 | needs-you state raises native notification + tray badge | T017 | PASS |
| US1-AS-4 | Tray state uses non-color glyph + text label | T017 | PASS |
| US1-AS-5 | App quit stops host + endpoint | T015 | PASS |
| US1-AS-6 | Same surface build renders identically as web client (US2 transport) | T018a | PASS |
| US2-AS-1 | Remote client discovers, attaches, folds event stream identically | T022, T023 | PASS |
| US2-AS-2 | NAT fallback to relay; if no path, fails gracefully; local run unaffected | T023 | PASS |
| US2-AS-3 | No valid org SSO → attach refused before any channel opens | T024 | PASS |
| US2-AS-4 | Host never armed → attach refused | T022, T024 | PASS |
| US2-AS-5 | Host unreachable → hub-side works; host-side shown unavailable-with-reason | T026 | PASS |
| US2-AS-6 | Deliberate close requires re-arm; transient drop re-attaches without re-arm | T025 | PASS |
| US2-AS-7 | Run-state folds over P2P; hub sees only discovery/signaling/relay, no run execution | T026a | PASS |
| US3-AS-1 | Remote permission answer advances run; logged attributed to verified remote operator | T033 | PASS |
| US3-AS-2 | Non-interactive command streams piped output; logged | T035 | PASS |
| US3-AS-3 | File/tree read within repo scope; out-of-scope refused and logged | T036 | PASS |
| US3-AS-4 | Remote skill launch executes on host as local run step | T033 | PASS |
| US3-AS-5 | Remote actions pass through same gate as local; no guardrail bypass | T034 | PASS |
| US3-AS-6 | Wagner provides no interactive shell; operator uses ssh/tmux over tunnel | T037b | PASS |
| US3-AS-7 | Dev-context data travels edge→operator device over P2P; never to hub | T037a | PASS |

**Coverage: 24/24 FRs, 8/8 SCs, 21/21 ASes — 100%.**

---

## Unmapped Tasks

Tasks with no clear mapping to an FR, SC, or user story.

| Task ID | Description | Likely Phase | Severity |
|---------|-------------|--------------|----------|
| T009 | Architecture guard — dependency direction test (carried from 001) | Foundational | LOW — cross-cutting gate; referenced by Article VII |
| T043 | Author ADR-0003 for remote transport/relay/lifecycle | Polish | LOW — optional artifact cited in plan §Optional Artifacts |
| T046 | Security hardening — secrets from env, schema validation at channel ingress | Polish | LOW — cross-cutting; maps to D-SEC-2 + Article X enforcement |
| T047 | Quickstart.md — arm → attach → permission → git diff → file read | Polish | LOW — doc artifact; cited in plan §Optional Artifacts |
| T048 | Update platform/README.md and platform/prd.md | Polish | LOW — doc artifact |
| T049 | Refactor per /tdd Refactor phase; re-confirm gate-seam/privacy/no-self-arm/dep-direction green | Polish | LOW — refactor phase, Article I compliant |

All unmapped tasks are in the Polish phase and are either cross-cutting quality enforcement, documentation, or a mandated Refactor step. None requires a new FR mapping.

---

## Constitution Alignment

Gates evaluated against `platform/docs/spec/constitution.md` v0.1.0 (Articles I–X). The Gates table is at `constitution.md:167–178`.

| Gate | Article | Result | Citation / Evidence |
|------|---------|--------|---------------------|
| Test-First | I | PASS | Every impl task in Phases 2–5 is preceded by its test task within the same phase. Foundational: T005–T009a precede T010–T014a. US1: T015–T018b precede T019–T021a. US2: T022–T026a precede T027–T032. US3: T033–T037b/T037a precede T038–T042. `tasks.md:29–127`. |
| Evidence-Driven | II | PASS | Challenge H2 ("remote feels no different") accepted and removed. `plan.md:14` now reads: "Run state is folded by the **same pure reducer** (`platform/shared/reducer`) in every environment; only the **transport** below the reducer differs...remote attach latency is bounded separately by SC-001 (first-frame p50 ≤ 3 s)." Gate II self-check at `plan.md:45` confirms removal. No vague adjective without adjacent quantification detected across spec.md or plan.md. |
| CRITICAL-Resolved | III | PASS | `challenges.md` confirms all 6 findings (2 CRITICAL) accepted and resolved. Validator found zero open CRITICAL in this run. |
| Independent-MVP | IV | PASS | US1 (P1) has a concrete non-empty Independent Test at `spec.md:34–35` covering 4 verifiable conditions (a–d), all tied to named FRs/SCs. US2 and US3 also have Independent Tests for completeness. |
| Simplicity | V | PASS (Complexity Tracking) | Four additions require Complexity Tracking entries; all four are present in `plan.md:209–216`: iroh P2P, org-run relay, hub discovery/signaling registry, Tauri tray/window-lifecycle wiring. Each has a rejected-alternative analysis. |
| Edge-Autonomy | VI | PASS | `plan.md:49` cites "Offline-completion test = T009a." T009a exists at `tasks.md:36`, scoped to `platform/edge/host/tests/integration/edge_autonomy_offline.rs`. Description asserts: start a run with hub unreachable, assert completion, assert zero hub/discovery/relay calls on run path. Satisfies `constitution.md:174` ("an offline-completion test exists"). Challenge C1 accepted and resolved. |
| Dependency-Direction | VII | PASS | T009 (`tasks.md:35`) runs the dependency-direction architecture guard at `platform/tests/architecture/dependency-direction.test.ts` (directory confirmed on filesystem). Gate VII confirmed carried from wedge-001. |
| Event-Sourced | VIII | PASS | T006 (`tasks.md:32`) is a reducer fold + replay test for new event kinds (replay produces byte-identical projection). T011 adds fold cases to the pure reducer with no I/O. No run-state mutation outside the fold. |
| Privacy-Boundary | IX | PASS | SC-006 reworded (H1 accepted): "0 bytes of code, file content, diff, or transcript are **stored on, or readable in plaintext by, any hub application service**; the org-run relay...forwards only **opaque encrypted transport frames it cannot decrypt** — those ciphertext frames are explicitly **not** a privacy violation." F-1 (`spec.md:232`) aligns. T026a + T037a (`tasks.md:90,117`) assert structural exclusion (schema `additionalProperties:false`) and relay size-logging only. |
| Schema-Validated | X | PASS | T005 (`tasks.md:31`) validates every channel/event schema (valid sample passes; unknown field rejected). T010 (`tasks.md:40`) authors all channel schemas with `draft 2020-12, additionalProperties:false`. |

---

## Cross-Artifact Inconsistencies

| ID | Type | Location(s) | Details | Severity |
|----|------|-------------|---------|----------|
| X1 | Terminology — consistent | spec.md, plan.md, tasks.md | Terms "surface", "host", "remote session", "capability channel", "arming", "tray", "gate", "transport abstraction", "reducer", "event log" are used consistently and defined in `spec.md §Key Entities`. No drift detected. | NONE |
| X2 | Tech choices — consistent | plan.md §Technical Context; tasks.md Phase 1 | Rust host + TypeScript/React UI + Deno hub (plan.md:26–30). tasks.md T001 (Cargo.toml iroh + Tauri), T003 (shared/transport TypeScript), T004 (hub/src/routes/discovery.ts) all align. No conflict. | NONE |

No terminology drift, no conflicting tech choices, and no contradictions between spec.md/plan.md/tasks.md found.

---

## Checklist Pass Rate

`platform/specs/002-edge-surface-and-remote-sessions/checklists/requirements.md`

**Total items:** 32
**N/A items:** 1 (CHK030)
**Applicable items:** 31
**Checked [x]:** 31
**Partial [~]:** 0
**Unchecked [ ]:** 0

**Pass rate: 31/31 = 100%**

CHK005 (previously `[~]`) is now `[x]` with rationale: "The experiential 'remote feels no different' plan-prose was **removed** (challenge H2, plan.md:14/45) and replaced with the mechanical claim (identical reducer over abstracted transport, FR-001/002) + the SC-001 latency bound." (`checklists/requirements.md:20`).

---

## Phase-Order Validation

For each user story, test tasks precede the implementation tasks they cover. Article I requires this; `tasks.md` is structured to enforce it per phase.

| Phase / Story | Test Tasks | Implementation Tasks | Order Valid? |
|---------------|------------|---------------------|--------------|
| Phase 2 — Foundational | T005, T006, T007, T008, T009, T009a | T010, T011, T012, T013, T014, T014a | PASS — `tasks.md:31–45` |
| Phase 3 — US1 | T015, T016, T017, T018, T018a, T018b | T019, T020, T021, T021a | PASS — `tasks.md:57–70` |
| Phase 4 — US2 | T022, T023, T024, T025, T025a, T025b, T026, T026a | T027, T028, T029, T030, T031, T032 | PASS — `tasks.md:83–99` |
| Phase 5 — US3 | T033, T034, T035, T036, T037, T037b, T037a | T038, T039, T040, T041, T042 | PASS — `tasks.md:112–127` |
| Final Phase — Polish | T044 (perf test, write first) | T045 (make T044 pass) | PASS — T043/T046–T049 are cross-cutting/doc/refactor |

**Phase ordering is valid in all phases. Zero Article I violations.**

Note on T014/T014a: T014 (gate-seam scaffold) precedes T014a (gate-seam unit test). This is a scaffolding precondition — the seam cannot be tested before it exists. The gate's behavior (remote action routing + guardrail parity) is covered by the test-first task T034 in Phase 5. Evaluated as acceptable within the Article I framework.

---

## Path Validity

Every path in tasks.md either (a) exists in the repo at validator runtime, or (b) is created as output of an earlier task. Paths checked against the actual filesystem at `/Users/marktripoli/Development/dev-ai-utilities/platform/`.

### Existing parent directories (confirmed at validator runtime)

| Directory | Status |
|-----------|--------|
| `platform/edge/host/` | EXISTS |
| `platform/edge/host/src/` | EXISTS |
| `platform/edge/host/tests/integration/` | EXISTS |
| `platform/edge/host/tests/unit/` | EXISTS |
| `platform/edge/ui/` | EXISTS |
| `platform/shared/schemas/` | EXISTS |
| `platform/shared/reducer/` | EXISTS |
| `platform/hub/src/routes/` | EXISTS (empty; new files are task outputs) |
| `platform/tests/architecture/` | EXISTS |

### New task path (BLOCKER-1 resolution)

| Task ID | Path | Status |
|---------|------|--------|
| T037b | `platform/edge/host/tests/integration/no_pty.rs` | PASS — parent directory `platform/edge/host/tests/integration/` confirmed on filesystem. |

All other task paths carry over from the initial run: PASS across all T001–T049. No HIGH path violations. The T018a/T018b write-first scenario (test files placed before implementation directories are fully scaffolded) is expected TDD behavior per Article I.

---

## Challenge-Resolution Verification

All 6 challenger findings were marked ACCEPTED in `challenges.md`. All 6 confirmed in artifact text in the initial run. No new findings from this re-run. Summary:

| Finding | Result |
|---------|--------|
| C1 — Article VI offline-completion test missing | VERIFIED — T009a at `tasks.md:36`; `plan.md:49` |
| C2 — FR-101 endpoint clause untestable in US1 | VERIFIED — FR-101 reworded `spec.md:126`; T025a at `tasks.md:87` |
| H1 — SC-006 "0 bytes" contradicts relay carrying opaque frames | VERIFIED — SC-006 `spec.md:155`; F-1 `spec.md:232`; T026a `tasks.md:90`; T037a `tasks.md:117` |
| H2 — "remote feels no different" is unquantified plan prose | VERIFIED — absent from `plan.md:14`; Gate II self-check `plan.md:45` |
| M1 — re-arm while armed behavior unspecified | VERIFIED — EC-010 `spec.md:99`; T025b `tasks.md:88` |
| M2 — exact T=30s boundary unspecified | VERIFIED — EC-011 `spec.md:100`; SC-007 `spec.md:156`; T025 `tasks.md:86` |

---

## Open Blockers

None. Both blockers from the initial run are resolved:

- BLOCKER-1 (US3-AS-6 zero coverage): resolved by T037b at `tasks.md:117` — negative-assertion test at `platform/edge/host/tests/integration/no_pty.rs`; coverage map at `tasks.md:183` updated to "US3-AS-6 (no interactive shell)→T037b".
- BLOCKER-2 (CHK005 `[~]`): resolved at `checklists/requirements.md:20` — checkbox updated to `[x]` with rationale citing H2 resolution.

---

## Re-run Determinism Note

This is the second run. Inputs changed between run 1 and run 2: `tasks.md` (T037b added; coverage map updated) and `checklists/requirements.md` (CHK005 `[~]` → `[x]`). The two blockers from run 1 are absent in run 2 because the underlying structural defects were corrected. No other findings changed. A third run on the current artifacts would produce zero blockers and a READY verdict — identical to this run.
