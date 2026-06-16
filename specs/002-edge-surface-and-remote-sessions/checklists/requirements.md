# Requirements Quality Checklist: Edge Surface & Remote Sessions

**Purpose:** Validate that the **requirements** are well-written, complete, unambiguous, and ready for implementation. This tests the spec itself, not code.
**Created:** 2026-06-15
**Spec:** [spec.md](../spec.md)

> Auto-validation pass run at end of Phase 1 (Specify). `[x]` = passes; `[ ]` = open (with reason); `[~]` = partial. Open items are expected before `/spec clarify` resolves the `CL-2xx` markers.

---

## Requirement Completeness

- [x] CHK001 All primary flows covered by a story (US1 tray-resident host + unified surface; US2 remote observe + hub-side; US3 remote act)? `[Completeness, Spec §User Scenarios]`
- [x] CHK002 Every FR has ≥1 acceptance scenario or success criterion tracing to it? `[Completeness]`
- [x] CHK003 All data entities listed (Surface, Host, Remote session, Capability channel, Tray presence, Operator identity, Run/Event, Hub)? `[Completeness]`
- [x] CHK004 All external dependencies named with failure modes (carried spine, P2P transport, hub, IdP, Tauri tray, CLIs)? `[Completeness, Spec §Dependencies]`

## Requirement Clarity

- [x] CHK005 Vague adjectives replaced by quantification or flagged? The experiential "remote feels no different" plan-prose was **removed** (challenge H2, plan.md:14/45) and replaced with the mechanical claim (identical reducer over abstracted transport, FR-001/002) + the SC-001 latency bound; latency/liveness targets are quantified in SC-001/007/008 (provisional, conditions fixed). `[Clarity, Article II]`
- [x] CHK006 Every FR a single testable statement (open quantifiers flagged, not compound)? `[Clarity]`
- [x] CHK007 Domain terms consistent across spec? Surface / host / remote session / capability channel / tray presence defined in §Key Entities; reused terms (operator, run, event, hub) point to wedge-001 + `platform/CONTEXT.md`. `[Clarity, Consistency]`

## Requirement Consistency

- [x] CHK008 Acceptance scenarios align with the FRs they reference? (US1-AS↔FR-100..105; US2-AS↔FR-200..212; US3-AS↔FR-300..305.) `[Consistency]`
- [x] CHK009 No contradictory requirements? Window-close-keeps-host (FR-101) vs quit-stops-host (FR-104) are distinguished, not contradictory (EC-007). `[Conflict]`
- [x] CHK010 Success criteria align with the user stories' value? `[Consistency, Spec §SC]`

## Acceptance Criteria Quality

- [x] CHK011 Every acceptance scenario Given/When/Then with measurable outcomes? Latency-bearing scenarios reference SC-001/007/008. `[Measurability]`
- [x] CHK012 Success criteria technology-agnostic? (No named transport/framework in SCs; "P2P channel", "hub boundary" are role terms, not products.) `[Measurability, Spec §SC]`
- [x] CHK013 Every SC verifiable without knowing the implementation? `[Measurability]`

## Scenario Coverage

- [x] CHK014 Happy-path scenarios covered (US1-AS-1/2/3, US2-AS-1, US3-AS-1/2)? `[Coverage]`
- [x] CHK015 Alternate-flow scenarios covered (host unreachable: US2-AS-5, EC-002; hub down: EC-008)? `[Coverage]`
- [x] CHK016 Error-flow scenarios covered (auth refused EC-003; un-armed attach EC-004; out-of-scope read EC-006)? `[Coverage]`
- [x] CHK017 Recovery/rollback covered (transient drop → re-attach US2-AS-6, SC-007; host returns → host-side re-attach EC-002)? `[Coverage, Edge Case]`

## Edge Case Coverage

- [x] CHK018 Zero-state defined (EC-002/EC-008 host/hub unreachable → hub-side floor / empty)? `[Edge Case]`
- [x] CHK019 Boundary defined (EC-007 window-close vs quit; SC-007 transient-drop ≤ 30 s)? `[Edge Case]`
- [x] CHK020 Concurrency defined (EC-005 two clients attach)? Resolution flagged CL-204 (single-controller vs first-write-wins). `[Edge Case]`
- [x] CHK021 Malicious/abuse defined (EC-003 bad token; EC-004 self-arm attempt; EC-006 out-of-scope/secret file read)? `[Edge Case]`

## Non-Functional Requirements

- [x] CHK022 Performance targets quantified for user-facing flows? SC-001 (attach p50 ≤ 3 s, NAT conditions), SC-008 (needs-you ≤ 5 s). Provisional, conditions fixed. `[Measurability]`
- [x] CHK023 Reliability target quantified? SC-004 (0 runs interrupted by window-close); SC-007 (≥ 95% transient re-attach). `[Measurability]`
- [x] CHK024 Observability (logs/metrics/traces) listed in plan.md? plan.md §1.3: 7 host log fields, 5 host + 2 hub metrics, 3 trace spans (remote.attach/control, devctx.cmd/file). `[Plan §Observability]`
- [x] CHK025 Security (authn/authz, trust boundary, secrets) in plan.md? plan.md §1.3 Security: dual identity (Ed25519 node + OIDC operator), owns-and-armed authz, repo-scope default-deny, secrets-from-env, F-1 zero-knowledge relay. `[Plan §Security]`
- [x] CHK026 Accessibility specified for user-facing flows? D-A11Y-1 carried to tray + mobile (FR-102, US1-AS-4, EC-009). `[Coverage]`

## Dependencies & Assumptions

- [x] CHK027 Every external dependency named with behaviour + failure mode? `[Spec §Dependencies]`
- [x] CHK028 Assumptions flagged in the Assumptions section, not buried in FRs? (Provisional SC targets listed under Assumptions.) `[Assumption]`
- [x] CHK029 Out-of-scope items explicitly listed (PTY/③, daemon, TUI, GUI-retire, cloud-exec, hub-exec, hostile-operator)? `[Spec §Out of Scope]`

## Ambiguities & Conflicts

- [N/A] CHK030 Fewer than 4 [NEEDS CLARIFICATION]? No marker budget in this skill version. All 5 `CL-2xx` markers resolved in §Clarifications Session 2026-06-15. `[Ambiguity]`
- [x] CHK031 /spec clarify run with answers integrated? CL-201 (org-run relay), CL-202 (repo-scoped default-deny), CL-203 (full-agency slice), CL-204 (first-write-wins), CL-205 (web-reach default) all resolved and applied throughout spec.md. `[Spec §Clarifications]`
- [x] CHK032 No sections referencing undefined concepts? (Surface, host, remote session, capability channel, tray presence, arming all defined; reused terms cite wedge-001 / ADRs.) `[Gap]`

---

## Notes

- Phase-1 auto-validation: 26/29 applicable items pass; 3 open by design (CHK024/025 await plan.md; CHK031 awaits `/spec clarify`).
- **Updated 2026-06-15 post-clarify + post-plan:** CHK024/025 now pass (plan.md §1.3); CHK031 passes (all CL-2xx resolved). 29/29 applicable items pass.
- Five `[NEEDS CLARIFICATION]` markers open: **CL-201** (relay own/public — high impact, security), **CL-202** (file-read scope — high impact, privacy), **CL-203** (slice-one capability scope — high impact, scope), **CL-204** (concurrent control — medium), **CL-205** (Windows/Linux desktop — medium, scope). High-impact three are the Phase-2 priority.
- No structural defects. Every FR traces to a story and to ≥1 acceptance scenario or SC. Every dependency has a failure mode. SC-002 / SC-003 / SC-006 are hard zeros.
- The carried 001 spine (gate, event log, reducer, identity, hub) is reused, not reinvented; this feature is additive (new surfaces + a remote path + new event kinds).

## Engineer Comments
