# Feature Specification: Shared Coding Runs & Learnings (the platform wedge)

**Feature Branch:** `001-shared-runs-and-learnings`
**Created:** 2026-06-14
**Status:** Draft (clarified — Session 2026-06-14)
**Input:** Engineer's direction (handoff 2026-06-14 + `platform/prd.md` §"Phase 1 — the wedge"): "shared coding runs + learnings — proves edge→hub sync + shared recall + identity. Needs no Temporal and only a minimal hub."

> **Spec authoring rules** (from `platform/docs/spec/constitution.md`):
> - Focus on **WHAT** users need and **WHY**. No HOW (tech stack, datastore, framework) — that lives in plan.md.
> - Every requirement is testable; vague adjectives without adjacent quantification fail Article II.
> - User stories are P1/P2/P3 and each P1 is independently shippable (Article IV).
> - No silent defaults: catalog defaults are cited in `## Defaults Applied`; everything else the engineer didn't specify is `[NEEDS CLARIFICATION]`.

---

## User Scenarios & Testing *(mandatory)*

The wedge proves the platform thesis end to end (`platform/prd.md` §"What the wedge proves"): **edge→hub sync** of run metadata + learnings under a **privacy boundary**, **shared recall** of prior learnings, and **verified operator identity**. Per Clarification CL-1, the wedge's definition of done includes **both** US1 and US2. Identity (verified) and project enrollment underlie both stories and are captured as Foundational requirements (FR-001..FR-004, FR-016..FR-017).

### User Story 1 — A run's learnings outlive its terminal (Priority: P1)

An edge operator launches and completes a coding run locally, exactly as the desktop app does today. When the run produces a learning and when it completes, the run's **metadata** (goal, status, outcome, guardrail usage, timing) and its **curated learnings** sync to the shared hub, attributed to the verified operator — while the codebase and the full agent transcript stay on the operator's machine. Sync happens only for **enrolled** projects. The org now accumulates an attributed, shared memory of what runs figured out, instead of that knowledge dying in one terminal.

**Why this priority:** This is the smallest slice that proves the platform's load-bearing mechanism (edge→hub sync across the privacy boundary, under verified identity and project enrollment). Shipping only this delivers a real, usable outcome — a browsable shared memory of runs and learnings the org did not have before — and every later capability (recall, workers, sensors) builds on this same sync spine. It is the independently-shippable MVP (Article IV); US2 ships in the same wedge release (CL-1) but is not required for US1 to deliver value.

**Independent Test:** An operator authenticates, enrolls a project, runs a coding run to completion on that project, and saves one learning; verify that (a) the run's metadata record and the learning appear in the hub attributed to that operator, and (b) no file contents or transcript text were transmitted — confirming verified attribution, enrollment-gated sync, and the privacy boundary end to end with no recall path required.

**Acceptance Scenarios:**

1. **Given** a verified operator identity, an enrolled project, and a reachable hub, **When** a coding run completes successfully, **Then** a run-metadata record (goal, terminal status, outcome reason, guardrail usage, start/end timestamps) is present in the hub attributed to that operator (sync is background/best-effort; latency per SC-006).
2. **Given** a run that has produced a learning whose `curation_state` is `curated` (the only shareable state — see Key Entities), **When** the learning is synced, **Then** the hub holds the learning's `{text, tags, project_id, created_at}` attributed to the operator, and a learning still in `captured` state does not sync (D-DATA-3).
3. **Given** a completing run, **When** sync runs, **Then** the transmitted payload contains only metadata + learning fields (validated against a declared schema with `additionalProperties: false`, Article X) and contains no file contents, diffs, or transcript text (Article IX).
4. **Given** the hub is unreachable, **When** a run completes, **Then** the run still completes locally, the sync is queued locally and retried later, and the operator is not blocked (Article VI, D-RES-2).
5. **Given** a run in progress, **When** an event is folded, **Then** run state is derived from the append-only event log via the pure reducer, and replaying the log reproduces the synced metadata snapshot (Article VIII).
6. **Given** an **unenrolled** project, **When** a run completes there, **Then** nothing syncs to the hub — the run is fully local, exactly as if no hub existed (FR-016).

---

### User Story 2 — Start a run informed by what the org already knows (Priority: P2)

When an operator starts a run in an enrolled project, the system surfaces the org's prior learnings relevant to the run's goal/area — to the operator and to the planning step — so the run begins informed rather than re-deriving what another run already established.

**Why this priority:** Recall is the operator-facing payoff, but it depends on US1 having synced learnings to the hub first, so it is sequenced after US1. It is separable: US1 ships a browsable shared memory without it. Per CL-1 it ships in the same wedge release as US1 (the wedge is not "done" until recall is proven), but US1 remains the independently-testable P1.

**Independent Test:** With at least one learning already in the hub (produced by a prior run on a related goal in an enrolled project), an operator starts a new run whose goal matches that learning's area; verify the relevant learning is surfaced to the operator before/at run start, is sourced from across the org's enrolled projects, and is not surfaced for an unrelated goal — confirming retrieval, relevance scoping, and org-wide-among-enrolled visibility.

**Acceptance Scenarios:**

1. **Given** a learning in the hub tagged for area X, **When** an operator starts a run whose goal concerns area X, **Then** that learning is surfaced at run start (recall latency per SC-006).
2. **Given** a learning for area X, **When** an operator starts a run on unrelated area Y, **Then** the learning is not surfaced — relevance is determined by **tag + goal-text match** (CL-4), returning the most recent matches up to the configured result cap.
3. **Given** the hub is unreachable, **When** an operator starts a run, **Then** recall returns empty and the run proceeds uninformed rather than blocking (Article VI, D-RES-2).
4. **Given** learnings authored by multiple operators across multiple **enrolled** projects, **When** recall runs for an enrolled project, **Then** results draw from all enrolled projects org-wide (CL-2), and never from unenrolled/personal projects (FR-016).
5. **Given** an operator working in an **unenrolled** project, **When** they start a run, **Then** recall returns empty (the project does not participate in the hub in either direction) (FR-016).

---

### Edge Cases

- **EC-001 (boundary):** A run produces a learning whose text exceeds the max learning size (provisional **8 KiB** — see Assumptions). Expected: the **save is rejected** with a clear message and the learning is **not** enqueued — no silent truncation (truncating would corrupt the lesson); the operator shortens and re-saves. The hub independently rejects an oversize `sync-learning` payload via schema `maxLength` (Article X). *(CL-7 resolved.)*
- **EC-002 (concurrency):** Two operators sync overlapping/near-duplicate learnings for the same area. Expected: the wedge **stores both** with no dedup; duplicate consolidation is deferred beyond the wedge (CL-6, Assumption).
- **EC-003 (failure):** The hub accepts a metadata record but the network drops before the learning syncs. Expected: partial sync is retried to completion; metadata and learning are eventually consistent; no partial state is presented as complete (D-RES-1, D-RES-2).
- **EC-004 (zero-state):** An operator starts a run in an enrolled project when the hub holds no learnings for any area. Expected: recall returns empty and the run proceeds; no error surfaced (D-RES-2).
- **EC-005 (privacy):** A saved learning's text contains a code snippet or secret. Expected: learnings are **operator-authored** (operator-initiated save, CL-3), so content is operator-reviewed at creation; the curation gate (D-DATA-3) is the control. Automatic content screening/redaction is deferred beyond the wedge.
- **EC-006 (identity):** A run attempts to sync with no verified operator identity. Expected: sync is **blocked** (Article IX requires attribution; verified identity per CL-2); the run still completes locally and the sync stays queued until identity is established.
- **EC-007 (enrollment):** A project's enrollment is revoked after learnings have already synced. Expected: revocation **stops all future sync and removes the project from future recall**; **already-synced records remain** in the hub — withdrawal/redaction of past records is out of scope for the wedge (CL-8 resolved).
- **EC-008 (enrollment precondition):** A project has **no git origin remote**. Expected: it **cannot be enrolled** — it has no stable cross-operator `project_key` — so it runs fully local (no sync, no recall), exactly as an unenrolled project (FR-016, B2). A monorepo behind a single remote collapses to one `project_key` (all its runs share one recall pool) — an accepted wedge limitation. *(B2, 2026-06-15.)*

---

## Functional Requirements *(mandatory)*

**Foundational — verified identity, enrollment, and the run/event spine (underlie US1 and US2)**

- **FR-001:** The system MUST attribute every synced run-metadata record and every synced learning to a **verified** operator identity (Article IX; D-IDENT-1; CL-2).
- **FR-002:** The system MUST establish operator identity by **server-verified authentication to the hub** (CL-2), specifically **OIDC SSO against the organization's IdPs (Google and JumpCloud)**: the hub validates an IdP-issued ID token (issuer / audience / signature) and gates access to verified **employees** by email domain / IdP group; `operator_id` is the IdP-issued subject. Identity is externally verified, never self-asserted. The Tauri edge uses the OIDC Authorization-Code + PKCE native-app flow; authentication is required only at the sync/recall boundary. *(Article V Complexity-Tracking item; mechanism per `platform/docs/adr/0002`.)*
- **FR-003:** Edge run state MUST be derived from an append-only event log folded by a pure reducer; replaying the log MUST reproduce the run snapshot (Article VIII). **Audit 2026-06-15 (F1): the carried `apps/wagner/src/store/reducer.ts` is a *UI projection* (folds operative events for the floor), and the carried host persists run state as an atomic *snapshot*, not an event log (`state/store.rs`, `run_loop.rs`). So the event-sourced run spine this FR requires is *wedge-built*, not carried — see tasks T006 (replay test) + T006a (run-event model + pure run reducer).**
- **FR-004:** Every run, event, transmission, and learning payload MUST validate against a declared JSON Schema (draft 2020-12, `additionalProperties: false`) before it is written, emitted, or synced (Article X; carried `apps/wagner/schemas/*.json`).
- **FR-016:** A project MUST be **explicitly enrolled** before any of its runs sync metadata/learnings to the hub *or* participate in recall. An unenrolled project runs fully locally — no sync, no recall — behaving exactly as if the hub were absent (CL-2; Article IX). This keeps personal/unenrolled projects out of the shared memory in both directions. A project's shared identity is its **`project_key` = normalized git origin remote**, derived at the sync/enroll boundary (B2); a project with **no git remote** has no stable cross-operator key and therefore cannot be enrolled — it runs fully local (EC-008).
- **FR-017:** Enrollment MUST be an explicit operator action tied to the verified identity (FR-002); the enrolled-project set MUST be inspectable by the operator. *(Article V Complexity-Tracking item — adds an enrollment registry to the hub.)*

**User Story 1 — sync metadata + learnings under the privacy boundary**

- **FR-005:** The system MUST run a coding run locally on the operator's subscription `claude`/`codex` CLIs, setting no metered LLM API key (Article VI; D-SEC-1; carried `apps/wagner`).
- **FR-006:** On run completion **in an enrolled project**, the system MUST sync the run's metadata — goal, terminal status, outcome/halt reason, guardrail usage (iterations used, cost used), and start/end timestamps — to the hub (D-DATA-1; FR-016).
- **FR-007:** The system MUST sync a run's learnings as `{uid, operator_id, project_key, text, tags, created_at, curation_state}` (D-STORE-3; the carried `MemoryRecord` shape with `user_id`→`operator_id` and the shared `project_key`), and MUST sync a learning only once it reaches the shareable `curation_state` `curated` — both `auto` and `captured` stay local (D-DATA-3).
- **FR-008:** The default (and only) sync path MUST NOT transmit file contents, file diffs, or full agent transcripts to the hub — verifiable by inspecting the sync payload against its schema (SC-002). The wedge provides **no mechanism** to share code/diffs/transcripts; any future sharing would be a separate, explicit, per-item action and is out of scope (Article IX; D-DATA-1, D-DATA-2). *(Reworded per challenge C5: states the prohibition only; the sharing mechanism is out of scope, so Article IX's per-item-sharing verification is N/A for the wedge.)*
- **FR-009:** When the hub is unreachable, the system MUST complete the run locally, queue the sync locally, and retry later without blocking the operator (Article VI; D-RES-2).
- **FR-010:** State and queued-sync writes MUST be atomic (temp-then-rename); a partial write MUST NOT be visible to the hub or to other readers (D-RES-1).
- **FR-011:** A learning MUST be created by an **operator-initiated save** (carried `save_memory`, which writes the initial `curation_state`). The `curation_state` enum `{auto, captured, curated}` is **introduced by the wedge** (the carried field is an open `string` only ever set to `"auto"` — `memory.rs:138`). A learning becomes eligible to sync only when the operator performs an explicit **"mark shareable"** action that sets `curation_state` to `curated` (CL-3; D-DATA-3); the edge MUST NOT enqueue a learning for sync while its `curation_state` is `auto` **or** `captured`. A learning's text MUST be ≤ 8 KiB; an oversize save is rejected with a message, never silently truncated (EC-001, CL-7). Automatic learning extraction is out of scope for the wedge.

**User Story 2 — recall relevant prior learnings at run start**

- **FR-012:** On run start, the system MUST surface relevant prior learnings from **two distinct, labeled sources**: (a) **local recall** — the operator's own prior learnings for the current project, folded into the run goal as today (carried `commands.rs:467-479`, any `curation_state`); and (b) **org recall** — for an **enrolled** project, the hub's org-wide `curated` learnings relevant to the goal/area (US2; FR-016). The two sets are kept distinct (not merged); a hub result whose `uid` already appears in the local set is dropped (dedup). Org recall is surfaced to the operator and the planning step before the run begins working.
- **FR-013:** Relevance MUST be determined by **tag + goal-text match** (CL-4), returning the most-recent matches ordered by recency, up to a configured result cap (provisional cap: 10 — see Assumptions; final value a plan-phase detail).
- **FR-014:** Recall MUST draw from learnings across **all enrolled projects org-wide** (CL-2), and MUST NOT return learnings from unenrolled/personal projects (FR-016).
- **FR-015:** When the hub is unreachable, recall MUST return empty and the run MUST proceed uninformed rather than blocking (Article VI; D-RES-2).

---

## Success Criteria *(mandatory)*

- **SC-001:** 100% of completed runs in an enrolled project (with a reachable hub) result in a hub-side metadata record attributed to the correct verified operator. (US1 sync + identity.)
- **SC-002:** 0 file-content, diff, or transcript bytes appear in the hub for a default-path sync, measured by inspecting the transmitted payload against the sync schema. (Article IX privacy boundary — a hard zero, not a target.)
- **SC-003:** A learning synced by one operator from an enrolled project is retrievable by a relevant later run started by **any operator in any enrolled project** org-wide, where projects are matched by `project_key` (normalized git origin remote, B2). (US2 recall + cross-person sharing; CL-2.)
- **SC-004:** A run started, worked, and completed with the hub unreachable succeeds 100% of the time, with sync reconciled once the hub returns. (Article VI edge autonomy.)
- **SC-005:** Replaying a run's event log from empty reproduces a byte-identical metadata snapshot. (Article VIII event-sourced truth.)
- **SC-006:** Recall is surfaced at run start with **p50 ≤ 2 s**, measured against a hub holding ≤ 5,000 learnings over a local/LAN connection to a single-process hub. Sync is background/best-effort — never on a user-blocking path — and a queued sync is reconciled **within 60 s of the hub becoming reachable**. *(Targets provisional pending plan-phase perf work; the measurement conditions are fixed here so the criterion is testable — challenge C7/H1.)*
- **SC-007:** 0 runs from unenrolled projects produce any hub-side record, and 0 recall results for any run are sourced from an unenrolled project. (FR-016 enrollment boundary — a hard zero.)

---

## Key Entities *(include only when feature involves data)*

- **Run:** One goal-loop execution. Attributes: id, goal, status, phase, iteration, guardrails (max_iterations, blocked_timeout, cost mode/budget/used), outcome/halt_reason, timestamps. Carried from `wagner-run.v1` (`apps/wagner/schemas/run-state.schema.json`); floor vocabulary renamed per D-PROJ-3. Lifecycle: drafted → running → (met | halted_guardrail | aborted | paused).
- **Event:** One normalized, immutable entry in a run's append-only log; folds into run/agent state via the pure reducer. Carried from `wagner-event.v1`; `operative→agent`, `faction→engine class`, `district→activity` (D-PROJ-3).
- **Learning:** A durable, operator-authored lesson from a run. The **synced** record carries `{uid, operator_id, project_key, text, tags, created_at, curation_state}` — the carried `MemoryRecord` shape with `user_id`→`operator_id` and the shared `project_key` (the carried local `project_id` path stays edge-only; see Project enrollment). The unit the hub remembers and recall returns; record shape carried from `apps/wagner` `save_memory`/`recall_memory`. The **`curation_state` enum is introduced by the wedge** — the carried field is an open `string` only ever set to `"auto"` (`memory.rs:138`), so `{captured, curated}` was *not* carried (corrects challenge H4 — see Clarifications 2026-06-15). **`curation_state` ∈ `{auto, captured, curated}`**: `auto` (machine-suggested, local) and `captured` (operator-authored, local) **never sync**; a learning becomes `curated` only when the operator explicitly **marks it shareable**; **only `curated` syncs** (D-DATA-3).
- **Operator identity:** The verified person (an employee) a run/event/learning is attributed to; scopes sharing. Established by **OIDC SSO** against the org IdPs (Google/JumpCloud); `operator_id` = the IdP-issued subject (CL-2; ADR-0002). Distinct from an *agent* (a worker in a run's roster — carried `identity.rs`).
- **Project enrollment:** Per-project participation state in the shared hub, keyed by **`project_key` = the normalized git origin remote** (so every operator on the same repo agrees with no coordination); the carried local `project_id` (a filesystem path — `commands.rs:385`) stays edge-only and is never the shared key. Set by an explicit operator action under their verified identity; `project_key` is derived at the sync/enroll boundary. Gates both sync and recall (FR-016, FR-017). New entity introduced by this wedge.
- **Hub sync record:** The metadata+learning payload that crosses the edge→hub boundary; declared schema, metadata+learnings only (Article IX, X). New entity introduced by this wedge.

---

## Assumptions

- The edge run behaviour (goal loop, guardrails, transmissions, subscription-CLI execution) is **ported** from `apps/wagner`, not reinvented (`platform/prd.md` §"the engine is already built"; D-PROJ-4).
- The wedge uses no Temporal and no always-on worker tier; edge runs are event-sourced on the local log (D-PROJ-5).
- The hub is **Deno + Hono + SurrealDB** (ADR-0001) — two cooperating processes (the Deno service + a SurrealDB server), no message broker, no queue, no Temporal. It supports OIDC SSO auth (FR-002, ADR-0002) and an enrollment registry (FR-017), both Article-V Complexity-Tracking items. *(Resolves the D-STORE-5 / D-PROJ-6 placeholders and supersedes the earlier "minimal hub, technology TBD" per ADR-0001.)*
- Operators within the org are mutually trusted for the wedge (single-org deployment); a hostile-operator threat model is out of scope.
- **Provisional values to confirm in plan-phase** (recorded as Assumptions per Article II, not silent defaults): recall result cap = 10 (FR-013); recall p50 ≤ 2 s and best-effort background sync (SC-006); store-both/no-dedup for overlapping learnings (EC-002, CL-6); operator-initiated learning save with curation gate (FR-011, CL-3); max learning text = 8 KiB with reject-on-exceed (EC-001, CL-7).

### Defaults Applied

- `D-IDENT-1` — FR-001 (identity attribution on the sync boundary).
- `D-STORE-3` — FR-007, Key Entities (learning = `MemoryRecord`).
- `D-DATA-1` — FR-006, FR-008, SC-002 (metadata+learnings sync; code/transcripts stay local).
- `D-DATA-2` — FR-008 (extra sharing is explicit per-item).
- `D-DATA-3` — FR-007, FR-011, EC-005 (curation_state gates sync; the `{auto,captured,curated}` enum and the operator "mark shareable" transition are introduced by the wedge, not carried — see Clarifications 2026-06-15).
- `D-SEC-1` — FR-005 (no metered API key; subscription CLIs).
- `D-RES-1` — FR-010, EC-003 (atomic writes).
- `D-RES-2` — FR-009, FR-015, EC-004 (graceful hub-unreachable degradation).
- `D-STORE-2` / Article VIII — FR-003, SC-005 (append-only log, pure reducer).
- Article X — FR-004, SC-002 (schema validation at every boundary).
- `D-PROJ-3` — Key Entities (floor→platform vocabulary rename).
- `D-PROJ-4` — Assumptions (edge stack ported from `apps/wagner`).
- `D-PROJ-5` — Assumptions (no Temporal/workers in the wedge).

### Defaults Overridden

- **D-IDENT-2** (placeholder "self-asserted or verified — not decided") → **OIDC SSO** against org IdPs (Google/JumpCloud) (CL-2; ADR-0002).
- **D-STORE-5 / D-PROJ-6** (placeholders "hub datastore / language-deploy — not decided") → **SurrealDB** store on a **Deno + Hono** service (ADR-0001).

Recorded so the catalog placeholders can be updated by an engineer edit.

---

## Out of Scope

- Temporal, durable cloud workflows, and any always-on worker tier (Phase 2 — `platform/prd.md`).
- Sensors / automatic ingestion of the org's engineering surface (CI, alerts, PRs) (Phase 3).
- Syncing code, diffs, or transcripts (privacy default; deferred and gated on explicit opt-in — Article IX).
- New *kinds* of run (code review, incident, ops) — those are later runs over the same spine.
- Automatic learning extraction and automatic content screening/redaction (learnings are operator-authored in the wedge — CL-3).
- Semantic/embedding-based recall (tag+text only in the wedge — CL-4; semantic recall is Phase 3).
- Duplicate-learning consolidation (store-both in the wedge — CL-6).
- Personal/unenrolled projects — explicitly excluded from the hub in both directions (FR-016).
- A mechanism to share code/diffs/transcripts — the wedge only guarantees the prohibition (FR-008).
- Withdrawal/redaction of already-synced records when a project's enrollment is revoked (EC-007).
- The desktop floor visualization and benched components (GoalEntry/Console/AgentInspector/MissionBar) — retired, not ported.
- A hostile-operator / multi-tenant isolation threat model (wedge assumes a single trusted org).
- Peer-to-peer / iroh-QUIC learning sync (a hub-less architecture) — parked as a Phase-2+ strategic alternative, not the wedge (interrogation 2026-06-15).

---

## Dependencies

- **Subscription `claude`/`codex` CLIs:** provide edge execution. If absent/unauthenticated, a run cannot start (carried `apps/wagner` preflight). Failure mode: preflight blocks launch with a clear message.
- **The capability library** (`skills/`, `agents/`, etc.): in scope for the CLIs running in the repo. Consumed one-directionally (Article VII). Failure mode: missing skills degrade run quality but do not block the wedge.
- **The hub** (**Deno + Hono + SurrealDB**, ADR-0001; supports OIDC SSO auth + enrollment registry): provides the shared event store + recall + identity. Failure mode: unreachable → edge runs proceed, sync queues, recall returns empty (Article VI; D-RES-2).
- **The org IdP** (Google / JumpCloud, via OIDC; ADR-0002): verifies operator identity for sync/recall. Failure mode: IdP unreachable or token expired → the operator cannot (re)authenticate, so sync stays queued and recall returns empty; the run still starts and completes locally (Article VI).
- **The carried run engine** (`apps/wagner` schemas + `reducer.ts`): the ported spine. Failure mode: N/A (in-repo source).

---

## Constitution Addenda *(optional)*

- *(none — platform constitution Articles VI–X already cover the wedge's non-negotiables.)*

---

## Cross-Plugin Surfaces *(optional)*

Not a multi-plugin feature. The wedge spans two layers of the platform runtime (`platform/edge`, `platform/hub`) plus the ported `platform/shared` spine; it touches the capability library only as a one-directional consumer (Article VII). No `plugins/*` changes.

---

## Clarifications

### Session 2026-06-14

- **CL-1** (scope) → **Full wedge**: both US1 (sync, P1) and US2 (recall, P2) ship in the wedge release; US1 remains the independently-shippable MVP. Applied to US1/US2 "Why this priority".
- **CL-2** (identity + visibility) → Identity is **server-verified hub authentication** (mechanism a plan decision); recall visibility is **org-wide across enrolled projects**, with an explicit **per-project enrollment** gate so personal/unenrolled projects never participate (sync *or* recall). Applied to FR-001, FR-002, FR-014, FR-016, FR-017, SC-003, SC-007, EC-006, EC-007, Key Entities, Defaults Overridden (D-IDENT-2).
- **CL-4** (relevance) → **Tag + goal-text match**, recency-ordered, capped (provisional 10). No embeddings. Applied to FR-013, US2-AS-2.
- **CL-3** (learning creation) → **Operator-initiated save** with curation-state gate; automatic extraction and content screening out of scope. Applied to FR-011, EC-005.
- **CL-6** (dedup) → **Store both**, no dedup in the wedge. Applied to EC-002.

### Remaining minor markers (plan-phase, low impact)

- **CL-5** → recall/sync latency: measurement conditions now fixed in SC-006 (recall p50 ≤ 2 s at ≤5k learnings/LAN; sync reconciled within 60 s of reachability); exact targets provisional pending plan-phase perf work.
- **CL-7** → **resolved**: max learning text = **8 KiB** (provisional, overridable); oversize → save rejected, not truncated. Applied to EC-001, FR-011; enforced by the `sync-learning` schema `maxLength` and a boundary test (T019d).
- **CL-8** → **resolved**: enrollment revocation stops future sync/recall; already-synced records remain (withdrawal out of scope). Applied to EC-007, Out of Scope.

### Session 2026-06-15 (challenge dispositions)

Spec changes applied from `challenges.md` (all ACCEPTED): FR-008 reworded to the prohibition only (C5); `curation_state` values defined (H4 — **note: superseded 2026-06-15, see interrogation amendments below**); FR-011 adds the edge no-enqueue rule (C6); SC-006 measurement conditions fixed + "session" term removed (C7/H1); EC-007/CL-8 revocation rule (C4); "minimal hub" quantified (H3). New test coverage for C1/C2/C3/C6 and an un-enroll route for C4 are added in tasks.md; the Gate VI critical-path flow (H2) is added in plan.md.

### Session 2026-06-15 (interrogation amendments — `/interrogate-with-docs`)

Stress-testing the READY spec against the **carried `apps/wagner` code** (which the `/spec` pipeline never opened) surfaced five code-vs-spec contradictions; the resulting engineer decisions are applied above.

- **B1** — `curation_state` enum `{auto,captured,curated}` is **wedge-introduced** (carried code only ever writes `"auto"` — `memory.rs:138`), plus an explicit operator **"mark shareable"** transition (none existed in carried code).
- **B2** — `project_key` = **normalized git origin remote** (carried `project_id` is a local path — `commands.rs:385`); no-remote ⇒ cannot enroll (EC-008).
- **B3 / B3b** — hub = **Deno + Hono + SurrealDB** (ADR-0001; the carried edge already runs SurrealDB + BM25).
- **B-auth** — identity = **OIDC SSO** (Google + JumpCloud), employees-only (ADR-0002; overrides plan R-1).
- **B-recall** — **two labeled recall sources** (carried local fold + hub `curated` org-wide block + `uid` dedup).

**Correction to H4:** it was marked "RESOLVED" on a false premise — the `{captured,curated}` enum was asserted "carried" but is not. Now genuinely resolved by B1. Iroh/QUIC parked (Phase-2+). Glossary: `platform/CONTEXT.md`. ADRs: `platform/docs/adr/0001`, `0002`.

### Session 2026-06-15 (carried-claim audit — pre-`/execute-plan`)

The audit Update 21 owed was run before any T001 work; the highest-leverage carried claims were verified at file:line. **B1, B2, no-carried-learning-schema, `MemoryRecord` shape, the local recall fold, and reducer purity all held up.** Three were wrong/overstated; engineer dispositions:

- **F1 (Article VIII / FR-003 / SC-005) — run metadata is NOT event-sourced in carried code** (atomic snapshot via `state/store.rs`/`run_loop.rs`; `WagnerEvent`s are a transient UI projection). **Engineer chose to BUILD the event-sourced run spine in the wedge** (not amend Article VIII). FR-003 citation corrected above; tasks T004/T006 reframed + T006a added.
- **F2 — no carried BM25 ranking to "mirror"** (`recall()` is tag+recency; BM25 index defined-but-unused). SurrealDB hub decision stands; ADR-0001/R-2 rationale to be corrected; edge-local vs hub-org recall rank differently (the two labeled sources make this visible — FR-012). Handled in T029.
- **F3 — D-PROJ-3 rename collision** (`district→activity` collides with the existing `activity` field). Corrected mapping `district→stage` (activity unchanged), `operative→agent`, `faction→engine_class`, value `oracle→planner`. Applied in T003/T004.

Full evidence + the held-up claims: `tasks.md §Amendment — Carried-claim audit (2026-06-15)`. Backups: `platform/.backups/{spec,plan,tasks}-pre-audit-2026-06-15.md`.
