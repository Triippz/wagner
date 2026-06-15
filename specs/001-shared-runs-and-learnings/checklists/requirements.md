# Requirements Quality Checklist: Shared Coding Runs & Learnings

**Purpose:** Validate that the **requirements** are well-written, complete, unambiguous, and ready for implementation. This tests the spec itself, not code.
**Created:** 2026-06-14
**Spec:** [spec.md](../spec.md)

> Auto-validation pass run at end of Phase 1 (Specify). `[x]` = passes; `[ ]` = open (with reason). Open items are expected before `/spec clarify` resolves the `CL-*` markers.

---

## Requirement Completeness

- [x] CHK001 All primary flows covered by a story (US1 sync, US2 recall; identity = Foundational FR-001..004)? `[Completeness, Spec §User Scenarios]`
- [x] CHK002 Every FR has ≥1 acceptance scenario or success criterion tracing to it? `[Completeness]`
- [x] CHK003 All data entities listed (Run, Event, Learning, Operator identity, Hub sync record)? `[Completeness]`
- [x] CHK004 All external dependencies named with failure modes (CLIs, library, hub, carried engine)? `[Completeness, Spec §Dependencies]`

## Requirement Clarity

- [x] CHK005 Vague adjectives replaced by quantification or flagged? ("minimal hub" → now SurrealDB + Deno, ADR-0001; "relevant" → CL-4 tag+text). `[Clarity, Article II]`
- [x] CHK006 Every FR a single testable statement (open quantifiers flagged, not compound)? `[Clarity]`
- [x] CHK007 Domain terms consistent across spec/plan/tasks? D-PROJ-3 vocabulary applied uniformly across all three artifacts. Glossary at `platform/CONTEXT.md` (Learning, Curation state, Operator, Agent, Shared/Local project, Recall); the 2026-06-15 amend (SurrealDB/Deno, OIDC, `project_key`, two-source recall) applied consistently across spec/plan/tasks. `[Clarity, Consistency]`

## Requirement Consistency

- [x] CHK008 Acceptance scenarios align with the FRs they reference? `[Consistency]`
- [x] CHK009 No contradictory requirements? `[Conflict]`
- [x] CHK010 Success criteria align with the user stories' value? `[Consistency, Spec §SC]`

## Acceptance Criteria Quality

- [x] CHK011 Every acceptance scenario Given/When/Then with measurable outcomes? US1-AS-1 and US2-AS-1 reference SC-006 (recall p50 <= 2 s; background sync) which is now quantified in spec.md and confirmed by plan.md §Performance Goals. `[Measurability]`
- [x] CHK012 Success criteria technology-agnostic? `[Measurability, Spec §SC]`
- [x] CHK013 Every SC verifiable without knowing the implementation? `[Measurability]`

## Scenario Coverage

- [x] CHK014 Happy-path scenarios covered (US1-AS-1/2, US2-AS-1)? `[Coverage]`
- [x] CHK015 Alternate-flow scenarios covered (hub-unreachable: US1-AS-4, US2-AS-3)? `[Coverage]`
- [x] CHK016 Error-flow scenarios covered (EC-003, EC-006)? `[Coverage]`
- [x] CHK017 Recovery/rollback covered (EC-003 partial-sync retry; D-RES-1)? `[Coverage, Edge Case]`

## Edge Case Coverage

- [x] CHK018 Zero-state defined (EC-004 empty hub)? `[Edge Case]`
- [~] CHK019 Boundary defined? EC-001 (max learning size) named in spec.md:59; CL-7 is plan-phase bounded (spec.md:199-201). No FR or SC traces to this edge case; structurally sound to leave open per validation-report.md §Checklist Pass Rate. `[Edge Case]`
- [x] CHK020 Concurrency defined? EC-002 resolved: store-both, no dedup in the wedge (CL-6 resolved, spec.md:60, §Clarifications CL-6). `[Edge Case]`
- [x] CHK021 Malicious/abuse defined? EC-005 resolved: operator-initiated save with curation-state gate is the control; automatic screening deferred beyond the wedge (CL-3 resolved, spec.md:63, FR-011). `[Edge Case]`

## Non-Functional Requirements

- [x] CHK022 Performance targets quantified for user-facing flows? SC-006: recall p50 <= 2 s; sync background/best-effort, never user-blocking. Confirmed in plan.md §Technical Context (Performance Goals) and targeted by T033. `[Measurability]`
- [x] CHK023 Reliability target quantified? SC-004 (100% offline-completion). `[Measurability]`
- [x] CHK024 Observability (logs/metrics/traces) listed in plan.md? plan.md §1.3 Observability specifies log fields (8 named), hub metrics (4 named), edge metric (1 named), and trace spans (run.recall, run.sync). `[Plan §Observability]`
- [x] CHK025 Security (authn/authz, secrets, trust boundary) in plan.md? plan.md §1.3 Security covers trust boundary, bearer token authn, per-operator authz, secrets-from-env, privacy boundary enforcement. `[Plan §Security]`
- [x] CHK026 Accessibility specified for user-facing flows? D-A11Y-1 (WCAG 2.1 AA, state never color-alone) carried. `[Coverage]`

## Dependencies & Assumptions

- [x] CHK027 Every external dependency named with behaviour + failure mode? `[Spec §Dependencies]`
- [x] CHK028 Assumptions flagged in the Assumptions section, not buried in FRs? `[Assumption]`
- [x] CHK029 Out-of-scope items explicitly listed? `[Spec §Out of Scope]`

## Ambiguities & Conflicts

- [N/A] CHK030 Fewer than 4 [NEEDS CLARIFICATION]? This skill version removed the 3-marker budget (`SKILL.md` Phase 1 §6: "no 3-marker budget — every gap is flagged"). 6 `CL-*` markers are open by design; `/spec clarify` resolves the high-impact ones next. `[Ambiguity]`
- [x] CHK031 /spec clarify run with answers integrated? CL-1/2/3/4/6 resolved and applied throughout spec.md (§Clarifications, Session 2026-06-14). CL-5/7/8 recorded as plan-phase bounded items in the same section. `[Spec §Clarifications]`
- [x] CHK032 No sections referencing undefined concepts? (hub, `curation_state ∈ {auto,captured,curated}`, `project_key`, operator, learning all defined in spec + `platform/CONTEXT.md`; the planner schema is explicitly out of wedge scope.) `[Gap]`

---

## Notes

- **Updated 2026-06-15 by spec-validator** after plan.md and tasks.md were written. 30/31 applicable items now pass.
- **Amended 2026-06-15 (`/spec amend`, post-`/interrogate-with-docs`):** curation enum `{auto,captured,curated}` + mark-shareable transition (B1), `project_key` = git origin remote (B2, EC-008), hub = Deno + SurrealDB (B3, ADR-0001), OIDC SSO (B-auth, ADR-0002), two-source recall (B-recall). These corrected five carried-claim contradictions the original pass missed. **Re-validation required.**
- CHK019 remains partial (~): EC-001 (max learning size, CL-7) has no FR or SC dependency; plan-phase bounded. Structurally sound for READY verdict.
- No structural defects. Every FR traces to a story and to at least one task. Every dependency has a failure mode. SC-002 and SC-007 are hard zeros. All 10 constitution gates pass.
- **Recommended next step:** re-run `/spec validate` (the 2026-06-15 amend added behaviour-changing tasks), then `/execute-plan`. Spec-challenger pass (Gate III) confirmed before the amend; the new tasks (T009b/T014b, T019e/T024a, T028b, OIDC T008/T013) are all TDD-ordered (test before impl).

## Engineer Comments
