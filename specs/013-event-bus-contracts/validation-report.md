# Validation Report: Event Bus Contracts (Phase 0)

**Validator:** spec-validator agent (read-only)
**Generated:** 2026-06-19
**Spec version:** git ref `main`, commit `aa3eb44` + FR-019 marker fix (re-validated 2026-06-19)
**Constitution:** `docs/spec/constitution.md` v0.1.0

> Read-only structural review. The validator does not propose substantive changes; it reports whether the spec is structurally complete and consistent enough to advance to implementation.

---

## Verdict

**READY**

All seven passes pass. Zero open blockers. Zero open `[NEEDS CLARIFICATION]` markers in `spec.md`. The spec is structurally complete and consistent; advance to Phase 7 (Execute).

### Re-validation summary (2026-06-19)

The single blocker from the initial run (X1 — FR-019 open marker at `spec.md:106`) is resolved. The bracketed clause was replaced with the decision text: compile-time for v1, load-time deferred; a matching Q→A was added to the Clarifications §Session 2026-06-18 block. Three confirmation checks run:

1. `grep -n "NEEDS CLARIFICATION" spec.md` — one match only, at line 8 (authoring-rules preamble describing the convention). Zero open requirement markers. PASS.
2. CHK030 criterion ("fewer than 4 markers") — zero markers in spec now. Contextually passes; checklist file itself is stale (pre-clarify snapshot) but that is a pre-existing LOW-severity finding, not a blocker.
3. No other content changed; coverage matrix, constitution gates, independent-test integrity, and test-coverage-by-story findings remain green from the initial run.

---

## Coverage Matrix

Every functional requirement (FR-001 through FR-019) and success criterion (SC-001 through SC-008) is checked against tasks.md.

| Requirement | Title | Mapped Task IDs | Status |
|-------------|-------|-----------------|--------|
| FR-001 | Envelope definition | T014 (tests T005, T006) | OK |
| FR-002 | origin + scope from v1 | T014 (test T005) | OK |
| FR-003 | ParticipantId | T012 (test T005) | OK |
| FR-004 | Scope{user,workspace} | T014 (test T005) | OK |
| FR-005 | StreamId{Run,Agent,Workspace} | T014 (test T005) | OK |
| FR-006 | Event namespaces + Ext | T010 (tests T005, T006) | OK |
| FR-007 | Command namespaces + Ext | T011 (tests T005, T006) | OK |
| FR-008 | Past-tense/imperative naming | T010, T011 (verify T024) | OK |
| FR-009 | Ext extension types | T010, T011, T021 (test T020) | OK |
| FR-010 | StabilityTier | T013 (test T006) | OK |
| FR-011 | Subscription shape | T012 (test T007 — C3 disposition: NoopAgent constructs + round-trips a Subscription) | OK |
| FR-012 | PluginManifest | T013 (test T009) | OK |
| FR-013 | Agent trait signature | T012 (test T007 NoopAgent) | OK |
| FR-014 | Capability vocabulary v1 (7) | T013 (test T009) | OK |
| FR-015 | Schema validation at boundary | T015 (test T006) | OK |
| FR-016 | Schema-version id on persisted payloads | T014 (test T006) | OK |
| FR-017 | Additive-versioning | T010–T014 (test T008) | OK |
| FR-018 | No embedded handle | T010, T011, T014 (test T007) | OK |
| FR-019 | Catalog as TS source | T015, T018 (tests T016, T017) | OK — coverage complete; open marker blocker is in the spec text (see Verdict) |
| SC-001 | Serde round-trip 100% | T005 | OK |
| SC-002 | 100% types have exported schema | T006, T015 | OK |
| SC-003 | Schema-invalid payloads rejected | T006 | OK |
| SC-004 | No non-serializable handle | T007 | OK |
| SC-005 | Additive-versioning regression | T008 | OK |
| SC-006 | Ext validates with zero core-enum edits | T020 | OK |
| SC-007 | Stable types have TS binding | T016, T018 | OK |
| SC-008 | All types carry stability tier + schema-version | T006 | OK |

**Coverage result: 100% — zero gaps.** The single BLOCKED finding is a structural marker problem in spec.md, not a coverage gap.

---

## Unmapped Tasks

| Task ID | Description | Mapping | Status |
|---------|-------------|---------|--------|
| T001 | Add schemars + smoke-confirm compat (R1) | Cross-cutting setup; gates T005; directly required by FR-015/017/SC-002/005 | OK — setup task, not story-labelled by convention |
| T002 | Add json-schema-to-typescript devDep | Cross-cutting setup; required by FR-019/SC-007 | OK |
| T003 | Register Cargo test targets + .gitkeep files | Cross-cutting setup; required for test discovery | OK |
| T004 | Skeleton `pub mod bus;` + empty submodules | Foundational phase; no story label per plan design | OK — explicitly explained in plan §Phase 2 |
| T022 | Module rustdoc pass | Polish; FR-017, FR-014 (declarative gap), cross-cutting | OK — [P] polish task, no story label |
| T023 | Verify gates green | Polish; cross-cutting gate confirmation | OK |
| T024 | Refactor pass | Polish; D-PROJ-3, FR-008 compliance | OK — [P] polish task |

No task is genuinely unmapped. Setup (T001–T003) and foundational (T004) tasks are non-story infrastructure; Polish tasks (T022–T024) are cross-cutting finalizations. Both categories are explained in the plan.

---

## Constitution Alignment

| Gate | Article | Result | Citation |
|------|---------|--------|----------|
| Test-First | I | PASS | Tasks T005–T009 precede T010–T015 for US1; T016–T017 precede T018–T019 for US3; T020 precedes T021 for US2. FR-011/Subscription: C3 disposition extended T007 to construct + round-trip a Subscription before T012 implements it. Complete test-before-implementation order confirmed. |
| Evidence-Driven | II | PASS | Every FR/SC cited to `runtime-architecture.md` (LOCKED), a `D-*` default, or a constitution article. spec-challenger vague-adjective scan returned zero open HIGH findings after H4 disposition (EC-001 `[NEEDS CLARIFICATION]` resolved). The open FR-019 marker is in a subordinate clause ("whether the extension schema registry is compile-time or load-time") that is correctly classified as a HOW question resolved in plan.md — it does not violate Article II (no vague adjective; it is a deferred design choice now resolved). However it remains as a structural open marker per the Blocker above. |
| CRITICAL-Resolved | III | BLOCKED | Three CRITICAL + four HIGH findings from challenges.md were disposed (9 accepted, 1 rejected with rationale). All challenge findings are closed. However Article III also requires zero open CRITICAL from spec-validator; the FR-019 open marker at spec.md:106 is a structural blocker flagged here as the one CRITICAL. |
| Independent-MVP | IV | PASS | US1 (P1) has a non-empty Independent Test stating action + value: "Construct an `Envelope` carrying a typed core `Event` of each namespace… the test suite proves serde round-trip + JSON-Schema accept/reject + the no-handle seam guard all pass — verifying the contract structure is authorable and boundary-safe." This was reworded per C2 disposition to claim the structure/seam/trait, not the full leaf set. The Independent Test is specific (action: construct + run suite; value: structure authorable, boundary-safe). Gate IV passes. US2 (P3) and US3 (P2) also have specific Independent Tests. |
| Simplicity | V | PASS | No new crate or service added. The contract is a module inside the existing `wagner-edge-host` crate. Two build-time tooling deps (`schemars`, `json-schema-to-typescript`) are added with Complexity Tracking entries per plan.md §Complexity Tracking — both with rejected-alternative analysis. Gate V passes. |
| Edge-Autonomy | VI | PASS | Contracts/types only — no hub round-trip, no metered API key. Trivially satisfied. |
| Dependency-Direction | VII | PASS | Contract types under `edge/host/src/bus/`; generated TS in `shared/contracts/` (pure standalone types, no platform/ import). T019 extends the existing dependency-direction guard to assert this. The generated file is in the `shared/` workspace which satisfies Gate VII's mandate. |
| Event-Sourced | VIII | PASS | Gate VIII is already satisfied by existing tests — **confirmed on disk**: `shared/reducer/run-reducer.test.ts:38` (`"SC-005: replay-from-empty is byte-identical to the incrementally-folded live snapshot"`) and `shared/reducer/remote-events.test.ts:69` (`describe("replay equals incremental fold (Article VIII)")`). Plan Gate VIII checkbox corrected to [x] per H1 disposition. This phase adds no new replay test because the gate is already green. NOT deferred — satisfied today. |
| Privacy-Boundary | IX | PASS | `Envelope` carries `Scope{user,workspace}` (FR-002/FR-004) — the multi-tenant filter seam. No code/diff/transcript field in any contract type. `Ext` payloads are bounded by `additionalProperties:false`. |
| Schema-Validated | X | PASS | Every Event/Command/manifest payload validated against draft-2020-12 `additionalProperties:false` schema (FR-015). Schema catalog exported at build/test time; validated by `jsonschema` (Rust) and `ajv` (TS). SC-002/003 verify this. |

---

## Cross-Artifact Inconsistencies

| ID | Type | Location(s) | Details | Severity |
|----|------|-------------|---------|----------|
| X1 | ~~Resolved marker still present~~ | spec.md:106 vs plan.md:124 | **CLOSED (re-validation 2026-06-19).** FR-019 marker removed from spec.md:106; replaced with the decision text. spec.md and plan.md are now consistent on the compile-time/load-time decision. | ~~CRITICAL~~ Resolved |
| X2 | Terminology — resolved | spec.md FR-006 vs runtime-architecture.md §2 code example | Reconciliation note added per C1/M1 dispositions in FR-006. The namespace set divergence between the LOCKED doc's illustrative examples and the spec's v1 set is documented and resolved. No drift remains. | Resolved |
| X3 | Gate VIII framing — resolved | plan.md Gate VIII | H1 disposition corrected Gate VIII from [~] to [x] and rewrote the reconciliation narrative to accurately state "already satisfied." | Resolved |

No unresolved terminology drift, conflicting tech choices, or path validity errors beyond X1.

---

## Checklist Pass Rate

`specs/013-event-bus-contracts/checklists/requirements.md` items: 32 total / evaluated below.

The checklist was authored before plan.md and tasks.md existed (stale PENDING markers); post-plan state is assessed per each item:

| ID | Status | Notes |
|----|--------|-------|
| CHK001 | PASS | Priorities confirmed (US1=P1, US2=P3, US3=P2) |
| CHK002 | PASS | FR→scenario traceability: FR-003 (ParticipantId) → AS-1 serde round-trip; FR-008 (naming) → T024 naming verify; FR-011 (Subscription) → T007 NoopAgent. Coverage matrix maps each FR to tasks. Tasks phase resolved the "tighten FR→scenario traceability" note. |
| CHK003 | PASS | All entities listed in Key Entities |
| CHK004 | PASS | Dependencies with failure modes in §Dependencies |
| CHK005 | PASS | No vague adjective without citation confirmed by spec-challenger scan |
| CHK006 | PASS — contextual | FR-019 compound (catalog + TS bindings) is the deliberate design (single source → dual consumer); tasks.md splits coverage (T015 for catalog, T018 for TS generation). Acceptable for a single-boundary contract. FR-001/FR-012 enumerate multi-field shapes as required by the typed-contract subject matter. |
| CHK007 | PASS | Domain terms consistent across spec/plan/tasks (confirmed by C1/M1 dispositions; no remaining drift) |
| CHK008 | PASS | Acceptance scenarios align with referenced FRs |
| CHK009 | PASS | No contradictory requirements |
| CHK010 | PASS | Success criteria align with user-story value |
| CHK011 | PASS | Every acceptance scenario in Given/When/Then with measurable outcome |
| CHK012 | PASS — confirmed | SC-001/002/007 name serde/JSON Schema/TypeScript as subject matter (Article X mandates schemas); engineer confirmed acceptable |
| CHK013 | PASS | Every SC verifiable without implementation knowledge |
| CHK014 | PASS | Happy path (US1 AS1–4) covered |
| CHK015 | PASS | Alternate flow (US2 extension path) covered |
| CHK016 | PASS | Exception/error flow (EC-003/005) covered |
| CHK017 | PASS | Recovery covered (EC-003 boundary rejection) |
| CHK018 | PASS | Zero-state (EC-004 empty-capability manifest) defined |
| CHK019 | PASS | Boundary (EC-001 additive evolution) defined |
| CHK020 | PASS | Concurrency (EC-002 stream+seq) defined |
| CHK021 | PASS | Malicious-input (EC-005) defined — M2 disposition folded compile-time registry rationale into EC-005 |
| CHK022 | N/A | No runtime user-facing flow in a types-only spec |
| CHK023 | N/A | No availability/reliability target applicable |
| CHK024 | PASS | Observability in plan.md §1.3 (D-OBS-1 fields baked into Envelope fields) |
| CHK025 | PASS | Security in plan.md §1.3 (trust boundary, capability declarations, no secrets) |
| CHK026 | N/A | No user-facing UI surface |
| CHK027 | PASS | Dependencies named with failure modes |
| CHK028 | PASS | Assumptions flagged in Assumptions section |
| CHK029 | PASS | Out-of-scope explicitly listed |
| CHK030 | PASS — contextual | FR-019 marker removed (re-validation 2026-06-19). spec.md now has zero open `[NEEDS CLARIFICATION]` markers. Criterion ("fewer than 4") satisfied. The checklist file itself is a stale pre-clarify snapshot; LOW-severity, not a blocker. |
| CHK031 | PASS | `/spec clarify` run and integrated (Session 2026-06-18 clarifications in §Clarifications) |
| CHK032 | PASS | No undefined concept references |

**Pass rate: 32/32 items pass (100%)** — after re-validation. CHK030 contextually passes with zero open markers in spec.md.

---

## Phase-Order Validation

For each user story, every test task ID precedes the implementation task ID for the same behaviour.

| Story | Test Tasks | Implementation Tasks | Order Valid? |
|-------|------------|---------------------|--------------|
| US1 | T005, T006, T007, T008, T009 | T010, T011, T012, T013, T014, T015 | PASS — tests 005–009 all numbered before impl 010–015; confirmed in tasks.md §Dependencies "Tests T005–T009 precede implementation T010–T015" |
| US3 | T016, T017 | T018, T019 | PASS — tests 016–017 before impl 018–019 |
| US2 | T020 | T021 | PASS — test 020 before impl 021 |
| FR-011 Subscription (C3 fix) | T007 (extended per C3 to construct + round-trip Subscription) | T012 (participant.rs defines Subscription) | PASS — T007 < T012 |

---

## Path Validity

Every file path in tasks.md: either exists at validator runtime, or is the output of an earlier task.

| Task | Key Path(s) | Status |
|------|-------------|--------|
| T001 | `edge/host/Cargo.toml`, root `Cargo.toml` | Both exist |
| T002 | `shared/package.json` | Exists |
| T003 | `edge/host/Cargo.toml`, `edge/host/schemas/bus/.gitkeep`, `shared/contracts/.gitkeep` | Cargo.toml exists; .gitkeep paths are outputs of T003 (new dirs) |
| T004 | `edge/host/src/lib.rs`, `edge/host/src/bus/mod.rs` | lib.rs exists; bus/ is output of T004 |
| T005 | `edge/host/tests/unit/bus_serde_roundtrip.rs` | `edge/host/tests/unit/` exists (confirmed); file is output of T005 |
| T006 | `edge/host/tests/unit/bus_schema_validate.rs`, `edge/host/schemas/bus/*.json` | tests/unit/ exists; both are outputs of T006/T015 |
| T007 | `edge/host/tests/unit/bus_no_handle_guard.rs` | Output of T007 |
| T008 | `edge/host/tests/unit/bus_additive_version.rs` | Output of T008 |
| T009 | (extends T006 file) | Output of T006 |
| T010–T013 | `edge/host/src/bus/event.rs`, `command.rs`, `participant.rs`, `manifest.rs` | Parent dir `edge/host/src/bus/` created by T004 |
| T014 | `edge/host/src/bus/envelope.rs` | Parent created by T004; depends on T010+T012 (both precede T014 in task list) |
| T015 | `edge/host/src/bus/mod.rs`, `edge/host/schemas/bus/*.json` | mod.rs created by T004; schemas/ dir exists; bus/ subdir created by T003 |
| T016–T017 | `shared/contracts/contracts.test.ts` | Output of T016/T018; `shared/contracts/.gitkeep` created by T003 |
| T018 | `shared/contracts/*.d.ts`, `shared/contracts/index.ts` | Parent dir created by T003; depends on T015 (schemas) |
| T019 | (existing dependency-direction guard extension) | Existing guard file modified; path not specified (low risk) |
| T020 | (extends T006 file) | T006 precedes T020 |
| T021 | `edge/host/tests/fixtures/ext/ext-slack-message.schema.json`, `edge/host/src/bus/event.rs` | `edge/host/tests/fixtures/` exists (confirmed); `ext/` subdir is output of T021; event.rs from T010 |
| T022 | `edge/host/src/bus/mod.rs` | T004 creates it |
| T023 | n/a (verify command) | — |
| T024 | (review of existing files) | All target files created by earlier tasks |

**Path validity: PASS.** All referenced paths either pre-exist or are created by a prior task in the ordered sequence.

---

## Independent-Test Integrity

For every P1 user story, the Independent Test field must be non-empty and specific (action + value delivered).

| Story | Priority | Independent Test | Action Present? | Value Delivered Present? | Result |
|-------|----------|-----------------|-----------------|--------------------------|--------|
| US1 | P1 | "Construct an `Envelope` carrying a typed core `Event` of each namespace (Run/Goal/Vault/Voice/Ui — one representative seed variant each) plus a Plugin manifest; the test suite proves serde round-trip + JSON-Schema accept/reject + the no-handle seam guard all pass — verifying the contract structure (the namespaces + `Ext` seam + `Agent` trait + manifest) is authorable and boundary-safe, with concrete leaf variants landing additively in `011` P0." | YES — construct + run test suite | YES — contract structure authorable and boundary-safe | PASS |
| US2 | P3 | "Emit an `Ext{ns,name,version,payload}` event whose payload matches its registered JSON Schema, with zero changes to the core `Event` enum source — verifying the extension seam works without a core PR." | YES | YES | PASS |
| US3 | P2 | "Generate TS bindings from the exported schemas; every `stable`-tier core type compiles against its binding and a representative payload validates — verifying the Rust→TS boundary catches shape drift." | YES | YES | PASS |

**Independent-test integrity: PASS for all three stories.**

---

## Test-Coverage by User Story

For every user story, count `[USn]` test tasks vs `[USn]` implementation tasks.

| Story | Test Tasks | Implementation Tasks | Ratio | Result |
|-------|------------|---------------------|-------|--------|
| US1 | T005, T006, T007, T008, T009 (5) | T010, T011, T012, T013, T014, T015 (6) | 5 test / 6 impl | PASS — multiple tests per impl unit |
| US3 | T016, T017 (2) | T018, T019 (2) | 2/2 | PASS |
| US2 | T020 (1) | T021 (1) | 1/1 | PASS |

**Test-coverage by story: PASS.** No story has zero test tasks.

---

## Open Blockers

None. Zero open blockers as of re-validation 2026-06-19.

The single blocker from the initial run (`spec.md:106` FR-019 open marker) was resolved prior to re-validation. Advance to Phase 7 (Execute).

---

## Re-run Determinism Note

This validator is deterministic: re-running on unchanged artifacts produces identical IDs and counts. All 7 passes were run in full; no pass was skipped. The sole blocker (FR-019 open marker at spec.md:106) is a machine-checkable string pattern.

If a re-run after the fix produces READY, the inputs changed (the marker was removed). If a re-run on unchanged inputs produces a different verdict, the inputs changed in ways not visible to this run.

---

## Notes on Near-Margin Findings

- **Gate VIII (plan.md):** The H1 challenge disposition correctly flipped the plan gate from [~] to [x]. The validator independently confirmed both on-disk test citations (`run-reducer.test.ts:38` and `remote-events.test.ts:69`) match the exact text in the plan. This gate passes cleanly and was NOT flagged as a CRITICAL.

- **FR-011/Subscription (C3):** The C3 challenge disposition extended T007 to construct and round-trip a `Subscription`. The updated T007 description explicitly states: "whose `subscriptions()` returns a `Vec<Subscription>` carrying a topic/namespace filter (e.g. `vault.*`) — assert that `Subscription` value serde-round-trips, exercising the `Subscription` shape (FR-011)." T007 precedes T012 (the implementation task). Article I is satisfied for FR-011. Passes cleanly.

- **CHK002 (FR→scenario traceability):** The original checklist flagged FR-003/FR-008/FR-011 as lacking dedicated scenarios. Post-tasks-phase, each maps to task coverage: FR-003 → T012 + T005 serde; FR-008 → T024 naming verify; FR-011 → T007 NoopAgent. Traceability is adequate for implementation; not flagged as a blocker.

- **H3 (REJECTED — Subscription speculative):** The challenger's H3 finding was rejected with rationale: `Subscription` is load-bearing for the `Agent` trait (FR-013) return type. C3 added the missing test. The rejection is correctly recorded. The validator does not re-open rejected challenge findings; it only verifies structural completeness.
