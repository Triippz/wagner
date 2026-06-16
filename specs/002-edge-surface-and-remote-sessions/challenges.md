# Spec Challenge Report: Edge Surface & Remote Sessions

**Reviewer:** spec-challenger agent
**Generated:** 2026-06-15
**Spec version:** feat/wagner-unified-composer (632e701)
**Constitution:** `platform/docs/spec/constitution.md` v0.1.0

> This report is adversarial by design. Findings without evidence are themselves CRITICAL.
> Engineer disposition is recorded per finding; rejected findings require rationale.

---

## Summary

| Severity | Count | Open | Resolved | Rejected (with rationale) |
|----------|-------|------|----------|---------------------------|
| CRITICAL | 2 | 0 | 2 | 0 |
| HIGH | 2 | 0 | 2 | 0 |
| MEDIUM | 2 | 0 | 2 | 0 |
| LOW | 0 | 0 | 0 | 0 |

**Original verdict: BLOCKED** — 2 CRITICAL. **All 6 findings ACCEPTED and resolved 2026-06-15 (engineer disposition below).** Re-validation required (Phase 6).

---

## Findings

| ID | Severity | Category | Location | Evidence | Recommendation | Engineer disposition |
|----|----------|----------|----------|----------|-----------------|----------------------|
| C1 | CRITICAL | Coverage-gap / Constitution-violation | tasks.md (all phases); constitution.md:102-104; plan.md:49 | See C1 detail below | Add a dedicated integration test task that starts a run and asserts completion with the hub **unreachable**. | **ACCEPTED.** Added **T009a** (Foundational): start an edge run with the hub unreachable + no remote client, assert completion + zero hub/discovery/relay calls on the run path. plan §Gate VI now cites T009a; coverage map updated. |
| C2 | CRITICAL | Coverage-gap / Contradiction | tasks.md:57; spec.md:124-125; tasks.md:169 | See C2 detail below | Make the FR-101 remote-endpoint-survival clause provably tested; fix the coverage map. | **ACCEPTED.** FR-101 reworded: host+log survival is US1; the **endpoint-survival clause is US2** (where the endpoint exists). T015 scoped to host+log; added **T025a** (armed endpoint survives window-close) in US2; coverage map FR-101→T015+T025a. Resolves the phase-ordering contradiction. |
| H1 | HIGH | Untestable-acceptance / Contradiction | spec.md:153; spec.md:230; plan.md:95-96 | See H1 detail below | Reword SC-006 to the real invariant (no hub-readable plaintext; encrypted relay frames excluded). | **ACCEPTED.** SC-006 + F-1 reworded: hard zero is **hub-readable run-bearing content**; the org-run relay is a distinct **zero-knowledge** forwarder of opaque ciphertext (size-logged only). Verification method changed from "inspect hub-bound traffic" to "(a) hub storage gains no code/diff/file field + (b) relay logs sizes only." Tests T026a/T037a aligned. |
| H2 | HIGH | Vague-adjective | plan.md:14 | See H2 detail below | Remove "remote feels no different" from plan-prose. | **ACCEPTED.** Removed; replaced with the mechanical claim (identical reducer over abstracted transport, FR-001/002) + the SC-001 latency bound. Gate II self-check corrected (the earlier "not an adjective" defense was wrong). |
| M1 | MEDIUM | Edge-case-missing | spec.md EC-001..009; FR-201/202 | See M1 detail below | Add an edge case for arm-while-already-armed; choose the behaviour. | **ACCEPTED.** Added **EC-010**: re-arm is idempotent-refresh (refresh NodeId/ticket+expiry, single new `remote.armed`, no duplicate registration), edge-only. Test **T025b**. |
| M2 | MEDIUM | Edge-case-missing | spec.md:154,165 | See M2 detail below | Define the exact T=30s transient-drop boundary. | **ACCEPTED.** Added **EC-011**: exclusive upper bound — `< 30 s` re-attaches without re-arm, `≥ 30 s` requires re-arm (TTL uses `< 30s`). SC-007 reworded; T025 now tests both sides of the boundary. |

---

## Finding Detail

### C1 — CRITICAL — Coverage-gap / Constitution-violation

**What the problem is:**

Article VI of the constitution (`platform/docs/spec/constitution.md:102-104`) states:

> "an integration test starts and completes an edge run with **the hub unreachable**"

This is an explicit gate criterion listed in the Article VI verification row of the Gates table (`constitution.md:174`):

> "plan.md shows no hub round-trip on the edge-run critical path; an **offline-completion test exists**; no API-key env var set"

No task in tasks.md satisfies this gate. Searching every task in Phases 1–5 and the Final Phase:

- T016 (`tasks.md:58`): "Run-survives-window-close test — a run executing when the window closes runs to completion" — this tests window-close, not hub-unreachable.
- T026 (`tasks.md:86`): "Hub-side-without-host test — with the host unreachable, a remote client browses learnings" — this tests a client with no host, not an edge run with no hub.
- T026a (`tasks.md:87`): "No-run-state-via-hub test — during an attached session, run-state events fold over the P2P channel; hub-bound traffic carries zero run-bearing content" — this tests the privacy boundary during a live session, not an offline edge run.
- T046 (`tasks.md:133`): "Security hardening" — not a run-completion test.
- T049 (`tasks.md:136`): "Refactor per /tdd Refactor phase" — not an offline-completion test.

plan.md:49 acknowledges the gate: "[x] Gate VI — Edge-Executes-Hub-Remembers" and states "Offline/remote-absent completion test planned." The word "planned" is not a task. No task ID is assigned to this test anywhere in tasks.md.

**What could go wrong:** Without this test, the gate self-certification is false. A future change could introduce a synchronous hub call on the run critical path (e.g., a lookup in the discovery registry that blocks) without any test catching it. The Article VI guarantee — offline-first execution — becomes untested and unverifiable, which is exactly the failure mode the constitution exists to prevent.

**What needs to change:** Add a task in the Final Phase (or insert into Phase 2 as a blocking foundational test) that: (a) starts the edge host with hub routes blocked (firewall rule, mock returning 503, or no hub process), (b) starts a run, (c) asserts the run progresses to completion, (d) asserts zero hub calls were made on the run's execution path. This must be a Red test that is confirmed failing before the code that makes it pass exists (Article I).

---

### C2 — CRITICAL — Coverage-gap / Contradiction

**What the problem is:**

spec.md:124-125 states FR-101 in full:

> "Closing the desktop **window MUST background** the app to the menu bar / system tray rather than exiting; the host process, its append-only event log, **and its remote endpoint** MUST keep running while the window is closed (R4)."

The coverage map at tasks.md:169 maps FR-101 to T015/T016/T017/T018. T015 (`tasks.md:57`) reads:

> "Host lifecycle test — `WindowEvent::CloseRequested` hides (does not exit); the host process + **event log** keep running after window-close; app-quit stops them"

T015 does not mention the remote endpoint. The test description covers "host process + event log" — two of the three required behaviors in FR-101. The third required behavior — the remote endpoint keeping running — is absent from T015's stated pass criteria.

The iroh endpoint implementation is T027 (`tasks.md:91`), which lives in Phase 4 (US2). T027 reads: "Implement the iroh endpoint + per-ALPN channels + discovery-assisted connect/relay-fallback in `platform/edge/host/src/remote/endpoint.rs`." This is the implementation of the remote endpoint. If the endpoint is not implemented until Phase 4, there is no remote endpoint for T015 (Phase 3) to test.

This creates a contradiction: FR-101 is a US1 requirement (Phase 3) that requires a remote endpoint to survive window-close, but the remote endpoint is not implemented until Phase 4. The coverage map claims T015 covers FR-101, but T015's description omits the remote endpoint clause entirely.

**What could go wrong:** The US1 checkpoint (`tasks.md:71`) states "Remote endpoint alive and ready for US2." But if T015 never tests remote endpoint survival, there is no failing test that would catch a regression where the endpoint shuts down on window-close. Phase 4 implementors would assume the endpoint lifecycle was already tested. It was not.

**What needs to change:** Either (a) rewrite T015 to explicitly include "and the iroh endpoint is live and accepting connections" — but this requires an iroh endpoint to exist before Phase 4, so T027 must split or be moved; or (b) add a new test task in Phase 3 that asserts the endpoint-stub/contract survives window-close, with the full iroh wiring deferred but the lifecycle contract tested against the stub; and update the coverage map for FR-101 accordingly. The coverage map entry at tasks.md:169 must also be corrected.

---

### H1 — HIGH — Untestable-acceptance / Contradiction

**What the problem is:**

SC-006 (`spec.md:153`) states:

> "**0 bytes** of code, file content, diff, or transcript cross the edge→**hub** boundary as a result of any remote session — remote dev-context payloads travel edge→operator-device over P2P only, verified by inspecting hub-bound traffic during a remote dev-context session."

The relay is co-located in the hub package (`plan.md:95-96`, project structure shows `platform/hub/src/relay/`). FR-203 (`spec.md:118`) mandates relay fallback: "The remote transport MUST be...with **relay fallback** when a direct path cannot form." When the relay is used, encrypted QUIC frames originating from dev-context payloads — file bytes, stdout/stderr, diffs — traverse the infrastructure at `platform/hub/src/relay/`.

SC-006's test method ("inspecting hub-bound traffic during a remote dev-context session") would detect non-zero bytes crossing the hub infrastructure when the relay is active. The relay does not terminate or read the payload (F-1, `spec.md:230`: "the relay, if used, carries only opaque encrypted transport frames it cannot read"), but the bytes traverse it.

SC-006 as written states a hard zero ("0 bytes") for hub-boundary crossings. This claim is falsified by any relay session. The spec addresses the substance of this in F-1 (`spec.md:230`) — the relay is zero-knowledge — but SC-006's literal statement and its verification method contradict F-1's framing.

This is a contradiction between spec.md:153 (SC-006, hard-zero claim) and spec.md:230 (F-1, relay-carries-opaque-frames acknowledgment). The two passages quote directly: SC-006 says "0 bytes...cross the edge→hub boundary"; F-1 says "the relay, if used, carries only opaque encrypted transport frames it cannot read." Encrypted bytes crossing the hub boundary is not zero bytes crossing the hub boundary.

**What could go wrong:** A test written to verify SC-006 as literally stated — zero hub-bound bytes — will fail whenever the relay is active, making the success criterion either untestable (you must suppress relay) or permanently false. Worse, passing SC-006 requires disabling the relay during the test, which means the test never validates the actual production path.

**What needs to change:** Rewrite SC-006 to state the actual invariant: "0 bytes of code, file content, diff, or transcript are stored on or forwarded by the hub in **plaintext** as a result of any remote session; encrypted relay frames are not considered a privacy violation because the relay cannot read them (F-1)." Update the verification method to assert (a) hub-side storage gains no code/diff/file fields after a remote dev-context session (schema constraint already provides this structurally) and (b) the relay log records frame sizes only, never decrypted payload. Remove the "inspecting hub-bound traffic" verification because that method would detect relay frames and fail the literal criterion.

---

### H2 — HIGH — Vague-adjective

**What the problem is:**

plan.md:14 contains:

> "Remote feels no different" falls out of this — the reducer never knows which transport delivered the event.

Article II (`constitution.md:44-45`) states:

> "Vague adjectives (fast, scalable, secure, robust, intuitive, lightweight, modern, simple, efficient) MUST NOT appear without an adjacent quantification."

"Feels no different" is in the same class of forbidden vague language — it is an experiential, unquantified claim about user perception. The Gate II self-check in plan.md:45 reads: "remote feels no different" is grounded in FR-002 (same reducer, abstracted transport), not an adjective." This claim is false. "Grounded in FR-002" describes the mechanism; it does not quantify the claim. FR-002 (`spec.md:108`) makes no claim about how the experience "feels." The phrase "feels no different" appears as unquantified plan-prose regardless of whether the mechanism behind it is sound.

The constitution does not exempt experiential adjectives from the quantification requirement if the mechanism is correct. The Article II list is illustrative, not exhaustive — the prohibition is on adjectives without adjacent quantification. "Feels no different" has none.

**What could go wrong:** Implementors may interpret "feels no different" as a performance and UX mandate without a testable threshold, then disagree on whether it is satisfied. The very quantification that makes FR-002 testable (same reducer, abstracted transport) is in the FR, not adjacent to this plan phrase.

**What needs to change:** Remove "remote feels no different" from plan.md:14. The rationale — same reducer, transport abstraction seam — stands on its own as a technical justification for FR-001/002. Replacing the phrase with a forward citation to FR-001/002 and SC-001 (the p50 ≤ 3 s attach latency target) covers the intent without the vague adjective.

---

### M1 — MEDIUM — Edge-case-missing

**What the problem is:**

The arming flow mutates state: it registers the host's NodeId + signaling ticket in the hub discovery registry under the operator's identity, and emits `remote.armed` on the event log (`plan.md:118-119`). EC-001..EC-009 (`spec.md:91-99`) list nine edge cases for this feature. None covers the re-arm scenario: what happens when `arm` is called while the host is **already armed**.

The edge-case completeness scan requires that every primary flow that mutates state includes at least one of: zero-state, boundary, concurrency, failure, malicious-input. The arm flow mutates both the hub discovery registry and the event log. The boundary case — already-armed re-arm — is not addressed. EC-005 (`spec.md:95`) covers two concurrent remote clients, not the arm-while-armed scenario. T022 (`tasks.md:82`) tests arm is edge-only and emits `remote.armed`, but does not specify the re-arm behavior.

**What could go wrong:** Without a specified re-arm behavior, implementors must guess: does re-arm extend the registration (updating the NodeId/ticket), or does it fail with a conflict, or is it a no-op? Two implementors could choose differently. A remote client could also attempt to force a re-arm via a legitimate arm action after observing a `remote.armed` event, potentially with a different NodeId.

**What needs to change:** Add an edge case EC-010 (or renumber appropriately): "Arm while already armed: the operator calls `arm` when the host is already armed and the discovery registration is active. Expected: [specify: re-registration refreshes the NodeId/ticket and emits a second `remote.armed` event, OR the call is a no-op with no new event, OR the prior registration is torn down and replaced]." The spec must choose.

---

### M2 — MEDIUM — Edge-case-missing

**What the problem is:**

SC-007 (`spec.md:154`) specifies: "a **transient drop ≤ 30 s** while still armed re-attaches without re-arming ≥ 95% of the time." The ephemeral session lifecycle definition (`spec.md:165`) states the session lifecycle as `unarmed → armed → attached → (detached-transient → re-attached | closed → torn-down)`. plan.md:185 confirms the 30 s re-attach window.

The boundary behavior at exactly T=30s is unspecified. SC-007 says "≤ 30 s" — does a drop of exactly 30 seconds qualify as within the window, or does expiry happen at T=30s making T=30s a hard cutoff (drop must be strictly less than 30s)? If the re-attach window is enforced by a server-side TTL on the discovery registration, the boundary is implementation-defined. EC-001..EC-009 do not address this.

**What could go wrong:** A transient network interruption that takes exactly 30 seconds to recover may succeed or fail depending on whether the implementation uses `< 30` or `<= 30` for the expiry comparison. More importantly, the test task T025 (`tasks.md:85`) tests "a transient drop ≤ 30 s while armed re-attaches without re-arm" — if the test uses a 28-second drop, it passes regardless of whether the 30s boundary is correct. Without a specified boundary behavior, T025 cannot test it.

**What needs to change:** Add an edge case to EC-001..EC-009 specifying: "Transient drop at exactly the 30 s boundary: a drop lasting exactly 30 s. Expected: specify whether the re-attach window is inclusive (≤ 30 s succeeds) or exclusive (< 30 s succeeds; 30 s fails). Update SC-007 to match." T025 must be updated to include a test at the boundary value.

---

## Pass Coverage

| Pass | Findings | Notes |
|------|----------|-------|
| 1. Vague-adjective scan | 1 (H2) | Scanned spec.md and plan.md for: fast, slow, scalable, robust, secure, intuitive, simple, lightweight, modern, efficient, seamless, clean, elegant, friendly, reasonable, appropriate, graceful, "feels no different." spec.md:190 uses "graceful degradation" — this is an established engineering term with a defined meaning (fail to a reduced state) and is immediately followed by the specific behavior in D-RES-2/FR-204/EC-001/EC-008; not flagged. plan.md:14 "remote feels no different" — no adjacent quantification; flagged as H2. |
| 2. Contradiction scan | 1 (H1) | Pair-checked: FR-001↔US1-AS-6 (pass); FR-004↔SC-202 (pass); FR-101↔US1-AS-1 (pass — both name the remote endpoint); SC-006↔F-1 (H1 — hard-zero vs. relay-carries-frames); plan.md §Gate VI↔tasks.md (C1 — gate claims checked, task missing); spec.md §Out-of-Scope↔FRs (pass — no out-of-scope behavior appears in an FR); plan.md §Technical Context↔tasks.md (pass — no datastore mismatch); plan.md §Constitution Check↔constitution.md (C1 raised via constitution scan, not a separate finding here). |
| 3. Constitution-violation scan | 1 (C1) | Articles checked: I through X. Article I — test-before-impl ordering verified in tasks.md phases; test tasks precede impl tasks per phase (pass). Article II — vague-adjective (H2). Article III — see verdict. Article IV — US1 has a concrete Independent Test (pass). Article V — four Complexity Tracking entries present, all required additions covered (pass). Article VI — gate self-certified in plan.md:49 with "offline/remote-absent completion test planned"; no task ID assigned anywhere in tasks.md; gate criterion (`constitution.md:174`: "an offline-completion test exists") fails (C1). Article VII — dependency direction task T009 present (pass). Article VIII — reducer replay test T006 present (pass). Article IX — SC-006/T026a/T037a cover the privacy boundary; caveat noted in H1 (the SC wording is the issue, not the test coverage). Article X — schema validation T005/T010 present (pass). |
| 4. Speculative-feature scan | 0 | Checked every FR (FR-001..005, FR-100..105, FR-200..204, FR-210..212, FR-301..305). FR-001: US1-AS-6, US2 generally (R1). FR-002: US1/US2/US3 transport-blind surface requirement. FR-003: R5 capability availability. FR-004: US3-AS-5, D6. FR-005: Article X carried. FR-100: R3. FR-101: R4, US1-AS-1. FR-102: R6, D-A11Y-1. FR-103: US1-AS-3. FR-104: R4, EC-007. FR-105: US1-AS-2. FR-200: D5, ADR-0002. FR-201: D5, US2-AS-4, EC-004. FR-202: D5, US2-AS-6. FR-203: D3, CL-201. FR-204: R5, EC-002/008. FR-210: US2-AS-1. FR-211: R5, US2-AS-5. FR-212: US2-AS-3/4, EC-003/004. FR-301: US3-AS-1/4, D4/D6. FR-302: US3-AS-2, D4. FR-303: US3-AS-3, CL-202, EC-006. FR-304: US3-AS-5, D6. FR-305: US3-AS-7, F-1. All FRs trace to at least one user story or constitution article. Zero speculative features found. |
| 5. Test-coverage scan | 0 (C2 is a coverage gap within a claimed mapping, not a missing map entry) | Verified every FR, AS, and SC against the coverage map (tasks.md:169) and confirmed task existence. FR-101 maps to T015 — task exists, but T015's description does not cover the remote endpoint clause (C2). FR-101 is covered by a task; the task's scope is the defect. All other FR→task, AS→task, SC→task mappings verified: T005-T049 exist in tasks.md. SC-001→T044/T045 (Final Phase); SC-002→T033/T034/T036; SC-003→T024; SC-004→T016; SC-005→T026; SC-006→T006/T026a/T037a; SC-007→T025; SC-008→T017. The Article VI offline-completion test has no task (C1 — raised in Constitution-violation scan, not double-counted here as a separate test-coverage finding). |
| 6. Edge-case completeness scan | 2 (M1, M2) | Primary flows that mutate state checked: arm/disarm (emits remote.armed/disarmed, writes to hub registry); attach/detach (emits remote.attached/detached); remote-control (emits remote.control); dev-context command (emits dev_context.command); dev-context file read (emits dev_context.file_read or dev_context.refused). For each flow, checked EC-001..009 for: zero-state (arm: EC-004 un-armed checked; attach: EC-002 host lost; pass), boundary (arm re-arm: missing — M1; session drop at 30s boundary: missing — M2), concurrency (EC-005 two clients, CL-204 first-write-wins; pass), failure (EC-001 NAT/relay failure; EC-003 bad token; EC-007 quit; EC-008 hub down; pass), malicious-input (EC-003 bad token; EC-004 self-arm; EC-006 out-of-scope file read; pass). |
| 7. Independent-test integrity scan | 0 | US1 (P1 only): Independent Test at spec.md:34 states a concrete sequence — "An operator launches a coding run, closes the desktop window, and walks away. Verify that (a) the run continues to completion with the window closed..., (b) the tray icon reflects the run's current state..., (c) when the run raises a permission prompt with the window closed, a native notification + tray badge appear within the liveness threshold (SC-008), and (d) reopening the window shows the live run state folded from the host's event log with no loss." Non-empty, action-specific, value-delivering, tied to a measurable threshold (SC-008). Passes Article IV. US2 and US3 are P2/P3 — not subject to Article IV. Both have Independent Tests for completeness; neither was evaluated as required. |
| 8. Cross-plugin contract scan | 0 | spec.md:234-236 explicitly states "Not a multi-plugin feature." No Cross-Plugin Surfaces section with obligations. Zero plugins named. Pass. |

---

## Engineer Acknowledgement

I have reviewed every finding above and either:

- accepted it and updated the spec/plan/tasks (citing the commit hash), OR
- rejected it with a recorded rationale.

Signed: _____________________ Date: _________
