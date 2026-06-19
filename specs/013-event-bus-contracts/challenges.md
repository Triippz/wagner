# Spec Challenge Report: Event Bus Contracts (Phase 0)

**Reviewer:** spec-challenger agent
**Generated:** 2026-06-18
**Spec version:** git ref `main`, commit `aa3eb44` (latest at review time)
**Constitution:** `docs/spec/constitution.md` v0.1.0

> This report is adversarial by design. Findings without evidence are themselves CRITICAL.
> Engineer disposition is recorded per finding; rejected findings require rationale.

---

## Summary

| Severity | Count | Open | Resolved | Rejected (with rationale) |
|----------|-------|------|----------|---------------------------|
| CRITICAL | 3 | 0 | 3 | 0 |
| HIGH | 4 | 0 | 3 | 1 |
| MEDIUM | 2 | 0 | 2 | 0 |
| LOW | 1 | 0 | 1 | 0 |

**Verdict:** BLOCKED at review. **Post-disposition (2026-06-19): 0 open CRITICAL, 0 open HIGH → challenge gate met.** 9 findings ACCEPTED with artifact edits; 1 REJECTED with rationale (H3). Engineer ratified C2 (keep seeds + honest wording) and H4 (scope size to `011` P1). Proceed to Phase 6 (Validate).

---

## Findings

| ID | Severity | Category | Location | Evidence | Recommendation | Engineer disposition |
|----|----------|----------|----------|----------|----------------|----------------------|
| C1 | CRITICAL | Contradiction | `spec.md:FR-006` vs `runtime-architecture.md:120–130` | spec.md FR-006 locks namespaces as `Run, Goal, Vault, Voice, Ui`. The LOCKED `runtime-architecture.md` §2 Rust code example (lines 124–129) defines the `Event` enum as `Goal(GoalEvent), Agent(AgentEvent), Integration(IntegrationEvent), Scheduler(SchedulerEvent), Vault(VaultEvent), Ui(UiEvent)` — no `Run`, no `Voice`. `011` plan Step 0 (lines 52–53) resolves this to `Run, Goal, Vault, Voice, Ui` but does so without a recorded deviation from the LOCKED code example. The text at `runtime-architecture.md:145` partially confirms (`Goal`, `Run`, `Vault`, `Scheduler`, `Ui`) but omits `Voice` and includes `Scheduler`, which spec.md omits. Three different authoritative sources give three different namespace sets. The spec calls its set "LOCKED" citing a LOCKED doc whose own example contradicts it. | Add an explicit reconciliation section to `spec.md` (or amend `runtime-architecture.md` through its stated review process) that resolves which namespace set is canonical. The three sources must agree on the exact set before implementation begins. Record the decision and the rejected-variant rationale in either a Complexity Tracking entry or an explicit amendment note. Until reconciled this is a CRITICAL contradiction under the challenge protocol authority-order rule 6 (Reductio). | ACCEPTED — FR-006 reconciliation note added: §2 code/prose examples are illustrative (§10 defers the concrete set); Agent/Integration/Scheduler are ParticipantKinds (FR-003), not Event namespaces; M1 folded in. (spec.md FR-006) |
| C2 | CRITICAL | Independent-MVP-failure | `spec.md §US1 Independent Test` / `tasks.md Phase 3 T005–T009` | The US1 Independent Test reads: "construct an `Event` of each namespace (Run/Goal/Vault/Voice/Ui) plus a Plugin manifest." The UNRATIFIED reading decision (admitted in team handoff) defines this as constructing **one representative seed variant per namespace** — e.g. `RunFinished`, `GoalAdded`. `tasks.md T010` implements exactly this: "one representative past-tense seed variant per namespace." The Article IV gate (`constitution.md:68–75`) requires that implementing only US1 "MUST yield a viable MVP that delivers the user the value the spec promises." The stated value in US1 is "a typed contract a developer can author a participant against." A contract with a single seed variant per namespace cannot satisfy "the no-handle seam guard all pass — verifying the contract is authorable and boundary-safe" for future leaf variants that do not exist yet. The Independent Test as written says "an `Event` of each namespace" — the test is constructible, but whether it proves the contract is "authorable" for all leaves (the stated value) is not demonstrated; it proves only that the scaffold compiles. The validator's gate criterion (`constitution.md:76`: "non-empty Independent Test (action + value delivered)") is met in text, but the Action produces a stub and the "value delivered" claim is weakened by the hollow leaf set. Article IV's mandate — "viable MVP that delivers the user the value the spec promises" — is not met when the MVP contract omits all concrete events except placeholder seeds. This is a structural weakening of the Independent Test. | Either (a) commit to a first-cut leaf set (the `011` P0 derivation from the 7 `wagner://*` channels) as part of US1 so the Independent Test is non-trivially constructible, or (b) rewrite the US1 value statement to honestly describe what the MVP delivers: "a scaffolded, extensible namespace structure authorable only with seed placeholders; full leaf enumeration follows in `011` P0." If (b), rewrite the Independent Test to match the narrower claim. The current mismatch between stated value and implementable proof is an Article IV violation. | ACCEPTED (option b) — seed-variant approach retained (enumerating leaves now is speculative and contradicts §10 / FR-006-007 deferral); US1 Independent Test reworded to claim the authorable STRUCTURE + Ext seam, with concrete leaves landing additively in `011` P0. (spec.md US1) |
| C3 | CRITICAL | Test-coverage gap | `tasks.md §Coverage Matrix` / `tasks.md T014` / `spec.md FR-011` | `FR-011` requires `Subscription` to be expressible by topic/namespace + filter. The coverage matrix at `tasks.md:129` maps FR-011 → T012 only (implementation). There is no test task preceding T012 that covers the `Subscription` shape. `T007` (no-handle guard) includes a `NoopAgent` dummy `impl Agent` covering the trait signature (`FR-013`) but does not construct a `Subscription` or assert the filter field shapes. No test task covers the `Subscription` type. This is a direct Article I violation: production code MUST NOT be written before a failing test exists (`constitution.md:28`). The test-first gate (`constitution.md:167–170`) requires "test task IDs preceding the first implementation task ID for the same behaviour." T012 has no preceding test. | Add a test task before T012 — or extend T005/T006/T007 — to construct a `Subscription` value with a topic/namespace filter and assert it round-trips and matches the expected shape. The Coverage Matrix must be updated to cite that test task for FR-011. | ACCEPTED — T007 extended: `NoopAgent.subscriptions()` constructs + serde-round-trips a `Subscription` with a `vault.*` filter; Coverage Matrix FR-011 now cites T007. (tasks.md T007 + matrix) |
| H1 | HIGH | Contradiction | `plan.md §Article VIII reconciliation` vs `constitution.md §Gates` | The plan at `plan.md:56` asserts: "the Gate VIII *test* (reducer pure + replay-equals-snapshot) is exercised where the reducer is wired (`011` P1+ / the state plane), not in this contracts-only phase. No `## Complexity Tracking` entry is required — this is a scope boundary, not a gate violation." The Article VIII gate text (`constitution.md:133`) specifies: "the reducer has no I/O and is unit-tested in isolation; a test replays a run's event log from empty and asserts the projection equals the live snapshot." That test EXISTS at `shared/reducer/run-reducer.test.ts:38` (`"SC-005: replay-from-empty is byte-identical..."`) and at `shared/reducer/remote-events.test.ts:69` (`"replay equals incremental fold (Article VIII)"`). The plan's claim that this gate is "exercised at `011` P1+" is false — it is already satisfied by carried tests in `shared/reducer/`. The plan's reconciliation narrative is correct in substance (bus is transient, durable truth is loro CRDT + existing reducer) but the framing "out of scope by phase" misrepresents the gate status. The gate is not deferred; it already passes. This mismatch between the plan's narrative and the actual state creates reviewer confusion and could lead a future validator to incorrectly flag a CRITICAL when the test already exists. | Rewrite the Gate VIII reconciliation in `plan.md` to state accurately: "Gate VIII is already satisfied by `shared/reducer/run-reducer.test.ts` (Article VIII / SC-005 replay-equals-snapshot). This phase adds no replay test because the gate already passes; the contracts are designed to make future `Event`s foldable by the same pure-reducer pattern." Remove the "out of scope by phase" framing — the gate passes today. | ACCEPTED — verified both tests exist on disk; plan Gate VIII checkbox ([~]→[x]) and the reconciliation bullet rewritten to "already satisfied by `shared/reducer` tests", "out of scope by phase" removed. (plan.md Gate VIII / §reconciliation) |
| H2 | HIGH | Contradiction | `plan.md §Technical Context` / `tasks.md T001–T002` / `edge/host/Cargo.toml` / `shared/package.json` | `plan.md §Technical Context` lists `schemars` 1.x and `json-schema-to-typescript` as new dependencies. `tasks.md T001` adds `schemars` to `Cargo.toml`; `tasks.md T002` adds `json-schema-to-typescript` to `shared/package.json`. Neither dep exists in either file today: `edge/host/Cargo.toml` (lines 1–60, inspected) contains no `schemars` entry; `shared/package.json` (full file, inspected) contains no `json-schema-to-typescript` entry and no `gen:contracts` script. The plan and tasks acknowledge the manifest edits are needed (T001/T002 are marked `[P]`), so the tasks are correct. However, there is no task that verifies the chosen `schemars` version is actually draft-2020-12 compatible (plan R1 calls this out as an "unknown" but the plan also says the resolution is "locked": `schemars 1.x`). Schemars 1.x does support draft 2020-12, but `jsonschema 0.18` (already in `edge/host/Cargo.toml`) supports draft 2020-12 only partially and the three-way compat (schemars → jsonschema → ajv) is plan R1's stated open research question that must be resolved before implementation. No research task exists in `tasks.md` to produce the R1 confirmation before T005 (the first test task that depends on it compiling). | Add a research/confirmation task before T005 that resolves and records the three-way compat tuple (schemars version + jsonschema version + ajv version) for draft-2020-12 + adjacently-tagged enums. This task must be a `[P]` prerequisite for T005–T009. Without it, the RED phase tests will be written against an unconfirmed compat matrix and may fail for tooling reasons rather than the intended logic reasons. | ACCEPTED — compat smoke-confirm folded into T001 and made an explicit T005 prerequisite (Within-US1 deps updated); plan R1 reworded "Resolve"→"Confirm" (L1 folded in). (tasks.md T001 + deps; plan.md R1) |
| H3 | HIGH | Speculative-feature | `spec.md §FR-011` / `spec.md §User Scenarios` | FR-011 requires: "Subscriptions MUST be expressible by topic/namespace + filter (e.g. `vault.*`, `ext.slack.*`, `stream:<id>`), not by matching a single god-enum." No user story in spec.md (US1, US2, US3) requires, references, or motivates the `Subscription` type as part of its acceptance or Independent Test. US1's acceptance scenarios (AS-1 through AS-4) do not include constructing or validating a `Subscription`. US3's scenarios cover TS bindings only. FR-011 itself notes "This spec defines the `Subscription` type shape; matching behavior is `011` P1." The subscription filter syntax (`vault.*`, `ext.slack.*`, `stream:<id>`) is a non-trivial design choice with no story that drives it. Under pass 4 (Speculative-feature scan), an FR with no user story tracing to it is HIGH. The Agent trait in US1's acceptance AS-3 mentions a guard for non-serializable handles including proving the trait signature is implementable — but the `Subscription` shape is not the same as the `Agent` trait. `Subscription` stands alone as a requirement with no user-motivating story. | Either (a) add an acceptance scenario to US1 that requires constructing a `Subscription` value (e.g. "Given a participant subscribing to `vault.*`, When I construct a `Subscription` for it, Then it serializes and the filter compiles against the type"), or (b) move FR-011 to the Out of Scope section and note that the `Subscription` type shape will be defined in `011` P1 when the bus routing behavior lands. Do not define a non-trivial API surface without a story that forces the design. | REJECTED — `Subscription` is load-bearing, not speculative: the `Agent` trait (FR-013) returns `Vec<Subscription>` and US1's NoopAgent (AS-3) exercises it. FR-011 annotated with that motivation; C3 adds the missing test; the v1 filter grammar is kept minimal (matching = `011` P1). Cutting it would break the FR-013 trait signature US1 ships. |
| H4 | HIGH | Vague-adjective | `spec.md §EC-001` | EC-001 reads: "A core `Event`/`Command` enum carries a variant added after v1. Expected: older consumers ignore unknown optional fields; required fields are never added to a `stable` type (additive-versioning, `runtime-architecture.md` §2). [Concrete max-variant/size limits: [NEEDS CLARIFICATION: none specified]]" The `[NEEDS CLARIFICATION]` marker for "max-variant/size limits" remains open and unresolved in the final spec. The plan's Constitution Check at `plan.md:40–41` explicitly notes: "EC-001 size limits → left as a minor additive note." The challenge protocol's severity floors classify this as HIGH for an unresolved vague adjective in a boundary edge case that governs malicious-input behavior (pass 6 also applies). An unbounded message size at the event boundary is a potential DoS vector — any consumer that materializes the full payload before rejection (e.g. the JSON Schema validator) is exposed. The plan dismisses this without recording it in Complexity Tracking or providing a concrete note about what happens when a payload is very large. Article II (`constitution.md:40–45`) requires every constraint to be quantified or cited. The open marker is not a quantification. | Resolve EC-001 by one of: (a) specify a concrete max payload size (e.g. "max event payload MUST NOT exceed 1 MB; rejected at the boundary before validation — rationale: the `jsonschema` validator must not materialize an unbounded blob"), or (b) explicitly remove the limit concern from this phase and document in Complexity Tracking why a size limit is not needed until the bus wires up (`011` P1). Leaving it as `[NEEDS CLARIFICATION]` in a shipped spec violates Article II. | ACCEPTED — EC-001 marker resolved: payload-size enforcement scoped to the `011` P1 intake boundary (no boundary accepts external bytes in a types-only phase), with the validator-OOM rationale recorded. Article II satisfied via scoping, not a number. (spec.md EC-001) |
| M1 | MEDIUM | Terminology-drift | `runtime-architecture.md:145` vs `spec.md:FR-006` / `tasks.md T010` | `runtime-architecture.md:145` mentions `Scheduler` as a core namespace in text: "Core events/commands are concrete, namespaced Rust enums (`Goal`, `Run`, `Vault`, `Scheduler`, `Ui`, …)". spec.md FR-006 and tasks.md T010 omit `Scheduler` entirely from the v1 namespace set, replacing it with `Voice`. No reconciliation document explains what happened to the `Scheduler` namespace: was it moved to a participant kind only? Absorbed into another namespace? Planned as a future additive namespace? The absence is undocumented. A future implementer will read the LOCKED doc and be surprised that `Scheduler` does not exist in v1. | Add a sentence to spec.md (or plan.md §1.1) explicitly noting that the LOCKED doc's text-level mention of `Scheduler` as a namespace is superseded by the clarified v1 set (`Run, Goal, Vault, Voice, Ui`), and that scheduler-related events will land under `Run` or a future `Scheduler` namespace added additively. This is a terminology-drift finding, not a contradiction: the spec is consistent internally, but the LOCKED authority source leaves the `Scheduler` namespace hanging. | ACCEPTED — folded into the C1 FR-006 reconciliation note: `Scheduler` is a `ParticipantKind` (FR-003), scheduler facts land under `Run` or a future additive namespace. (spec.md FR-006) |
| M2 | MEDIUM | Edge-case-missing | `spec.md §EC-005` / `tasks.md T021` | EC-005 covers malicious-input (oversized/unexpected field). The plan's coverage at `plan.md §1.3` states: "An extension registers a schema, then emits a payload with an unexpected/oversized field. Expected: rejected by `additionalProperties:false` validation against the registered schema." T021 adds a fixture schema and payload. However, the edge case as defined in spec.md covers only field-level violations. A genuine malicious-input scenario for an extension schema system includes: a plugin **registering a schema that is itself malicious** — e.g. a JSON Schema that recursively references itself (causing the validator to loop), or a schema with deeply nested `$ref` cycles. Neither EC-005 nor any other edge case addresses malicious schema registration. The plan notes "no per-path or per-host scoping in v1" but does not address malicious schema payloads at registration time. Under pass 6, the "malicious-input" category is present for emitted payloads (EC-005) but absent for registered schemas. | Add EC-006: "A plugin registers a recursive or circular JSON Schema. Expected: the schema registry rejects it at registration time (cycle detection) or the validator is bounded (max evaluation depth), so a malicious schema cannot cause validator OOM or infinite recursion." If the compile-time schema registry makes this impossible (all schemas are compile-time artifacts), document that fact explicitly in EC-005 to close the gap. | ACCEPTED — EC-005 now states schemas are compile-time artifacts (FR-019 compile-time registry), so malicious load-time schema registration is not a v1 attack surface; cycle-detection / bounded depth revisited when a load-time registry lands. (spec.md EC-005) |
| L1 | LOW | Style | `plan.md §Phase 0 — Research` | The plan records R1 as "an unknown" but also states its resolution is "already locked": adjacently-tagged enums and `schemars 1.x`. The "Resolutions already locked by this plan" subsection (`plan.md:118`) contradicts the framing of R1 as a genuine unknown requiring research. If the resolution is locked, R1 is not an unknown — it is a decision that requires confirmation, which is different. The distinction matters because it determines whether a research task must be completed before T005 (if R1 is a genuine unknown) or whether R1 is just a documentation checkpoint (if the decision is already made). | Reframe R1 from "Resolve" (implying open) to "Confirm" (implying the decision is locked but the three-way compat must be verified by running it). This also makes H2's proposed research/confirmation task clearly scoped: it is a one-time verification task, not an open design decision. | ACCEPTED — folded into H2: R1 reframed as CONFIRM (decision locked; smoke-verify the three-way tuple in T001). (plan.md R1) |

---

## Categories

| Category | Floor | Findings in this report |
|----------|-------|------------------------|
| Constitution-violation | CRITICAL | — |
| Coverage-gap | CRITICAL | C3 |
| Independent-MVP-failure | CRITICAL | C2 |
| Test-coverage gap | CRITICAL | C3 |
| Contradiction | HIGH | C1 (CRITICAL by cross-constitution rule), H1, H2 |
| Untestable-acceptance | HIGH | — |
| Vague-adjective | HIGH | H4 |
| Speculative-feature | HIGH | H3 |
| Cross-plugin-undefined | HIGH | — |
| Terminology-drift | MEDIUM | M1 |
| Edge-case-missing | MEDIUM | M2 |
| Style / wording | LOW | L1 |

---

## Pass Coverage

| Pass | Findings | Notes |
|------|----------|-------|
| Vague-adjective scan | 1 | Words flagged: EC-001's `[NEEDS CLARIFICATION: none specified]` on max-variant/size limits. Other adjectives (`plain`, `concrete`, `bounded`, `stable`) were checked and all carry adjacent quantification or citation. H4 issued. |
| Contradiction scan | 3 | Pairs checked: (FR-006 ↔ runtime-architecture.md §2 code example), (FR-006 ↔ 011 plan Step 0), (plan.md Gate VIII "out of scope" ↔ existing shared/reducer tests), (plan.md R1 "unknown" ↔ plan.md "resolutions already locked"). C1 (namespace set) and H1 (Gate VIII framing) issued. H2 (missing manifest edits acknowledged but not gated by a research confirmation task) issued under contradiction scan. |
| Constitution-violation scan | 0 | Articles I–X checked. C3 is issued under Test-coverage gap (Article I), not as a free-standing constitution-violation finding, because the tasks.md does not explicitly list a test for FR-011/Subscription. H1 is issued under Contradiction, not constitution-violation, because the Article VIII gate is already satisfied by existing tests — the plan's narrative is wrong, not the spec. No article is violated without Complexity Tracking where Complexity Tracking is required. |
| Speculative-feature scan | 1 | FR-001–FR-019 checked against US1/US2/US3. FR-011 (Subscription shape) has no user story that requires it and its acceptance scenarios do not call it out. H3 issued. FR-013 (Agent trait) is motivated by US1 AS-3 (NoopAgent guard). FR-014 (Capability vocab) is motivated by US1 AS-4. All other FRs trace to at least one acceptance scenario or are cross-cutting infrastructure with a constitution/defaults citation. |
| Test-coverage scan | 1 | FR-001 through FR-019 and SC-001 through SC-008 checked against tasks.md Coverage Matrix. FR-011 (Subscription) maps to T012 (implementation only) with no preceding test task. C3 issued. All other FRs have at least one test task (T005–T009, T016–T017, T020) preceding the relevant implementation task. |
| Edge-case completeness | 1 | Edge cases EC-001 through EC-005 checked. Categories: zero-state (EC-004 ✓), boundary (EC-001 ✓ partially — size limit open), concurrency (EC-002 ✓), failure (EC-003 ✓), malicious-input (EC-005 ✓ for emitted payload; absent for registered schema). M2 issued for the malicious-schema-registration gap. |
| Independent-test integrity | 1 | P1 story: US1 only. Independent Test: present, non-empty, states action + value. However the action ("construct an `Event` of each namespace") constructs only seed stubs per the UNRATIFIED reading. C2 issued under Independent-MVP-failure — the stated value ("contract is authorable and boundary-safe") is not delivered by a stub-only contract. US2 (P3) and US3 (P2) Independent Tests are non-empty and specific. |
| Cross-plugin contract scan | 0 | spec.md does not have a `§Cross-Plugin Surfaces` section. No cross-plugin obligation rows to check. Pass produces zero findings. |

---

## Detailed Evidence for Key Findings

### C1 — Namespace set contradiction

`runtime-architecture.md:121–130`:
```rust
/// Namespaced, additive-only. Each variant lives in its own module.
pub enum Event {
    Goal(GoalEvent),
    Agent(AgentEvent),
    Integration(IntegrationEvent),
    Scheduler(SchedulerEvent),
    Vault(VaultEvent),
    Ui(UiEvent),
}
```

`runtime-architecture.md:145`:
> "Core events/commands are concrete, namespaced Rust enums (`Goal`, `Run`, `Vault`, `Scheduler`, `Ui`, …)"

`spec.md:FR-006`:
> "The contract MUST define a core `Event` enum namespaced into exactly these v1 namespaces: `Run`, `Goal`, `Vault`, `Voice`, `Ui`, plus `Ext`"

`specs/011-runtime-foundation/plan.md:52–53`:
> "the core `Event` enum (namespaced: `Run`, `Goal`, `Vault`, `Voice`, `Ui` — derived from today's 7 channels)"

Three sources, three different namespace sets:
- LOCKED code example: `Goal, Agent, Integration, Scheduler, Vault, Ui` (no Run, no Voice)
- LOCKED doc text: `Goal, Run, Vault, Scheduler, Ui` (no Voice, no Agent/Integration)
- spec.md FR-006 + 011 plan: `Run, Goal, Vault, Voice, Ui` (no Agent, Integration, Scheduler)

The spec cites `runtime-architecture.md §2` as the authority for FR-006, but that authority does not unambiguously support the claimed set.

### C2 — Independent-MVP-failure: stub contract vs claimed value

`spec.md §US1 Independent Test`:
> "Construct an `Envelope` carrying a typed core `Event` of each namespace (Run/Goal/Vault/Voice/Ui) plus a Plugin manifest; the test suite proves serde round-trip + JSON-Schema accept/reject + the no-handle seam guard all pass — verifying **the contract is authorable and boundary-safe**."

`tasks.md T010`:
> "Define `Event` in `edge/host/src/bus/event.rs` — adjacently-tagged enum over `Run | Goal | Vault | Voice | Ui` + `Ext { ns, name, version, payload }`, with **one representative past-tense seed variant per namespace** (e.g. `RunFinished`, `GoalAdded`, `NoteUpdated`, `UtteranceTranscribed`, `SurfaceFocused`); full leaf enumeration is deferred to `011` P0 and lands additively."

`constitution.md §Article IV`:
> "Every user story prioritised P1 MUST be independently testable: implementing only that story MUST yield a viable MVP that delivers the user the value the spec promises."

The value promised: "The contract makes 'create an agent' and 'add an integration' the identical authoring move."

A contract with seed stubs delivers the *structure* of that authoring move, not the move itself. Whether this crosses the Article IV threshold requires the engineer to confirm: does US1 deliver a viable MVP, or only the scaffolding for one?

### C3 — FR-011 Subscription: no test task

`tasks.md §Coverage Matrix (line 128)`:
> `FR-011 Subscription shape | T012`

T012 is an implementation task. The tasks.md Coverage Matrix has no test task entry for FR-011. T007 (no-handle guard, `bus_no_handle_guard.rs`) covers `Event`, `Command`, `Envelope` but not `Subscription`. T005 (serde round-trip) constructs `Envelope` carrying `Event` and `Command` but does not construct `Subscription`. No test task precedes T012 for this behavior.

`constitution.md §Article I`:
> "Production code MUST NOT be written before a failing test exists."

### H1 — Gate VIII plan narrative is factually wrong

`plan.md:56`:
> "the Gate VIII *test* (reducer pure + replay-equals-snapshot) is exercised where the reducer is wired (`011` P1+ / the state plane), not in this contracts-only phase."

`shared/reducer/run-reducer.test.ts:38–42` (on disk, confirmed):
```typescript
it("SC-005: replay-from-empty is byte-identical to the incrementally-folded live snapshot", () => {
  let live = foldRunEvent(null, completedLog[0]!);
  for (const e of completedLog.slice(1)) live = foldRunEvent(live, e);
  expect(JSON.stringify(replayRun(completedLog))).toBe(JSON.stringify(live));
});
```

`shared/reducer/remote-events.test.ts:69`:
```typescript
describe("replay equals incremental fold (Article VIII)", () => {
```

The gate test already exists and passes. The plan's claim it is "exercised at `011` P1+" contradicts the on-disk evidence.

### H2 — New deps unconfirmed in manifests with no research gate task

`edge/host/Cargo.toml` (inspected, lines 1–60): no `schemars` entry.
`shared/package.json` (inspected, full file): no `json-schema-to-typescript`, no `gen:contracts` script.

`plan.md §Phase 0 Research`:
> "R1 — Schema/codegen three-way compatibility on draft 2020-12. Resolve: a (schemars version, jsonschema 0.18, ajv 8 / json-schema-to-typescript) tuple that all agree on draft 2020-12 AND on the adjacently-tagged enum representation."

`tasks.md`: no research/confirmation task for R1 before T005. T005 is the first test that depends on `schemars` compiling.

---

## Engineer Acknowledgement

Every finding above has been disposed (2026-06-19):

- 9 ACCEPTED — spec.md / plan.md / tasks.md edited in the working tree (see the disposition column for the exact location of each edit). Backups in `.backups/013-challenge-disposition/`.
- 1 REJECTED with recorded rationale (H3 — `Subscription` is load-bearing for the FR-013 `Agent` trait).

Commit is deferred to the engineer's branch decision (specs/012 + specs/013 are still uncommitted on `main`; see handoff). Disposition applied by Claude under engineer ratification of C2 (keep seeds) and H4 (scope size to `011` P1).

Engineer: Mark Tripoli — ratified C2 + H4 via `/spec challenge` disposition, 2026-06-19.
