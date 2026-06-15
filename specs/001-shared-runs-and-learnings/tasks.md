# Tasks: Shared Coding Runs & Learnings (the platform wedge)

**Feature Branch:** `001-shared-runs-and-learnings`
**Inputs:** [spec.md](./spec.md), [plan.md](./plan.md), `platform/docs/spec/constitution.md` v0.1.0

> **Constitution Article I (NON-NEGOTIABLE):** every behaviour-changing implementation task is preceded by a test task that exercises it. Tests are written first and confirmed failing before implementation.

## Format

`- [ ] [TaskID] [P?] [USn?] Description with concrete path` — `[P]` = parallelisable (different files, no incomplete dep); `[USn]` only on user-story tasks.

---

## Phase 1 — Setup (Shared Infrastructure)

- [x] T001 Scaffold `platform/{shared,edge,hub}` packages — cargo workspace for `edge/host`; `package.json`+`tsconfig`+`vitest.config` for `shared` and `edge/ui`; `deno.json` for `hub` (Deno; ADR-0001).
- [ ] T002 [P] Add hub dependencies (`hono`, `surrealdb` JS SDK, `ajv`) + an OIDC client in `platform/hub/deno.json` (Deno runtime; ADR-0001/0002).
- [ ] T003 [P] Port carried JSON Schemas → `platform/shared/schemas/{run,event,transmission}.schema.json`, renaming floor vocabulary per D-PROJ-3 — **corrected mapping (audit F3, 2026-06-15): `operative→agent`, `faction→engine_class`, `district→stage` (NOT `activity`: the carried event already carries a distinct `activity` field — read/edit/test/think — that must stay), and the `district` enum value `oracle→planner`**. **Author** `learning.schema.json` fresh — no carried learning schema exists; the `MemoryRecord` shape is carried but `curation_state ∈ {auto,captured,curated}` + the 8 KiB `maxLength` are authored (B1, C5). The carried `oracle-plan.schema.json` (planner) is **out of wedge scope** — not ported.
- [ ] T004 [P] Port the carried **UI-projection** reducer + types → `platform/shared/reducer/{reducer.ts,types.ts}` (the operatives/transcript fold; no I/O, no UI dep) **and** port its existing unit test (`apps/wagner/tests/unit/reducer.test.ts`) with the corrected vocab. **Audit F1 (2026-06-15): this carried reducer is a UI projection, NOT the run-state source of truth — the run-metadata snapshot the wedge syncs is set wholesale via `applyRun(snapshot)`, not folded from the event log. The event-sourced run spine that Article VIII / FR-003 / SC-005 actually require is wedge-built — see T006 / T006a.**

---

## Phase 2 — Foundational (Blocking Prerequisites)

No user-story work begins until this phase is complete. Verified identity (OIDC), enrollment, `project_key` derivation, schema validation, the event-sourced spine, and the dependency-direction guard underlie both stories.

### Tests (write FIRST, confirm FAILING)

- [ ] T005 [P] Schema-validation tests for the new boundary schemas (valid sample passes; unknown field rejected via `additionalProperties:false`) in `platform/shared/schemas/boundary.test.ts` (Article X, D-TEST-3).
- [x] T006 [P] **Run-event replay test (write FIRST, confirm FAILING):** replaying a run's append-only **run-event** log from empty reproduces a byte-identical run-**metadata** snapshot (goal, status, halt_reason, iterations_used, cost_used, timestamps), in `platform/shared/reducer/run-reducer.test.ts` (Article VIII, FR-003, SC-005). **Audit F1: the carried edge persists run state as an atomic snapshot, not an event log (`state/store.rs`; `run_loop.rs` calls `state::save(&run)` each iteration); `WagnerEvent`s are a transient UI projection (`run_loop.rs` `emit` is "No-op in tests"). So this exercises the wedge-built run-event spine (T006a), not a port.**
- [ ] T007 [P] Architecture guard test: assert no file outside `platform/` imports `platform/`, and `install.sh` succeeds against a fixture repo with no `platform/`, in `platform/tests/architecture/dependency-direction.test.ts` (Article VII).
- [ ] T008 [P] **OIDC** auth contract tests (a valid IdP ID token mints a hub session; invalid/expired/wrong-audience → 401; non-employee domain/group → 403) in `platform/hub/tests/contract/auth.test.ts` (FR-002, ADR-0002).
- [ ] T009 [P] Enrollment contract tests (enroll idempotent; list enrolled; sync/recall refused with 422 when not enrolled) in `platform/hub/tests/contract/enrollment.test.ts` (FR-016, FR-017).
- [ ] T009a [P] Un-enroll/revocation contract test: un-enroll removes the project; subsequent sync/recall for it is refused/excluded; already-synced records remain queryable, in `platform/hub/tests/contract/revocation.test.ts` (EC-007, CL-8).
- [ ] T009b [P] Unit: `project_key` derivation — two different local paths with the same git origin remote yield one `project_key`; a repo with **no** remote yields none (→ cannot enroll) — `platform/edge/host/tests/unit/project_key.rs` (B2, FR-016, EC-008).

### Implementation

- [x] T006a Build the wedge's **event-sourced run spine** (makes T006 pass) — **new foundational work per audit F1, not a port.** A run-event model + types (`platform/shared/reducer/run-events.ts`): run-level events `run.created{goal,guardrails}`, `run.iteration_advanced`, `run.cost_folded{delta}`, `run.status_changed{status}`, `run.halted{reason}`, `run.completed{ts}`; and a **pure** run reducer (`platform/shared/reducer/run-reducer.ts`) folding them into the synced `RunMetadataSnapshot`. The ported edge orchestrator (run_loop) MUST emit these and **atomically append** them to a durable per-run log so the log is the source of truth and the snapshot is its projection (Article VIII, FR-003, SC-005; emission wiring lands with the orchestrator port in Phase 3, T024).
- [ ] T010 [P] Author boundary JSON Schemas → `platform/shared/schemas/{sync-run,sync-learning,recall-request,recall-response,enrollment,auth}.schema.json` (draft 2020-12, `additionalProperties:false`; metadata+learnings only — Article IX/X). Makes T005 pass.
- [ ] T011 Wire the shared `ajv` validate harness in `platform/hub/src/validate/index.ts` (validate every inbound/outbound body — Article X).
- [ ] T012 Initialise **SurrealDB** store + `operator` and `project_enrollment` tables + the BM25 analyzer (mirror the edge `wagner_en`) in `platform/hub/src/store/db.ts` (plan §1.1; ADR-0001).
- [ ] T013 Implement **OIDC** auth — `POST /sessions` ID-token validation (issuer/aud/signature + employee domain/group gate) + session middleware in `platform/hub/src/routes/auth.ts`, and the edge Authorization-Code + PKCE flow in `platform/edge/host/src/sync/auth.rs` (FR-002, ADR-0002). Makes T008 pass.
- [ ] T014 Implement enrollment (`POST /projects/enroll`, `GET /projects`) + a reusable enrollment-gate check in `platform/hub/src/routes/projects.ts` (FR-016, FR-017). Makes T009 pass.
- [ ] T014a Implement un-enroll (`DELETE /projects/enroll`) — removes the enrollment row so future sync/recall is gated out; already-synced records are retained (no withdrawal) in `platform/hub/src/routes/projects.ts` (EC-007, CL-8). Makes T009a pass.
- [ ] T014b Implement `project_key` derivation (normalize the git origin remote) at the sync/enroll boundary in `platform/edge/host/src/sync/project_key.rs`; enrollment, sync, and recall all carry it (B2, FR-016). Makes T009b pass.

**Checkpoint:** Foundation ready — verified identity, enrollment gate, validated boundary schemas, replayable spine, dependency-direction guard all green. User-story phases can begin.

---

## Phase 3 — User Story 1 — A run's learnings outlive its terminal (Priority: P1) 🎯 MVP

**Goal:** A completed run in an enrolled project syncs its metadata + operator-authored learnings to the hub under a verified identity, with code/transcripts never crossing the boundary, and degrades gracefully offline.
**Independent Test:** Authenticate, enroll a project, run a coding run to completion saving one learning; verify the metadata record + learning appear in the hub attributed to the operator, and that zero file/transcript bytes were transmitted.

### Tests (write FIRST, confirm FAILING)

- [ ] T015 [P] [US1] Contract test `POST /runs` — upsert run metadata (idempotent on `run_id`); 422 when project not enrolled; payload schema rejects any code/diff/transcript field — `platform/hub/tests/contract/runs.test.ts` (FR-006, SC-002, SC-007).
- [ ] T016 [P] [US1] Contract test `POST /learnings` — upsert (idempotent on `uid`); curation-state gate; 422 when not enrolled — `platform/hub/tests/contract/learnings.test.ts` (FR-007).
- [ ] T017 [P] [US1] Integration: run completes → metadata + learning synced, attributed to the verified operator, and **0** code/transcript bytes transmitted; also asserts no metered API key is set — `platform/edge/host/tests/integration/sync.rs` (SC-001, SC-002, FR-005, FR-008, US1-AS-1/2/3).
- [ ] T018 [P] [US1] Integration: hub unreachable → run completes locally, sync queued + retried to completion, operator not blocked — `platform/edge/host/tests/integration/offline_sync.rs` (SC-004, Article VI, US1-AS-4, EC-003).
- [ ] T019 [P] [US1] Integration: unenrolled project → nothing syncs to the hub — `platform/edge/host/tests/integration/enrollment_gate.rs` (SC-007, FR-016, US1-AS-6).
- [ ] T019a [P] [US1] Unit: an interrupted/partial sync-queue write is never visible to a reader (atomic temp-then-rename) — `platform/edge/host/tests/unit/queue_atomic.rs` (FR-010; challenge C2).
- [ ] T019b [P] [US1] Unit: a learning whose `curation_state` is `auto` **or** `captured` is NOT enqueued for sync; only `curated` is — `platform/edge/host/tests/unit/curation_gate.rs` (FR-011; challenge C6; B1).
- [ ] T019c [P] [US1] Test: a completed sync emits `wagner_sync_total{op,status}`, increments `wagner_sync_queue_depth` on enqueue, and opens a `run.sync` span — `platform/edge/host/tests/integration/sync_observability.rs` (plan §1.3; challenge C3).
- [ ] T019d [P] [US1] Boundary test: a learning whose text exceeds 8 KiB is rejected at save (edge) and at `POST /learnings` (hub via schema `maxLength`), never truncated — `platform/edge/host/tests/unit/learning_size.rs` + `platform/hub/tests/contract/learnings.test.ts` (EC-001, CL-7, FR-011; challenge M1).
- [ ] T019e [P] [US1] Unit: the operator **"mark shareable"** action transitions a learning to `curated` (from `auto` or `captured`); an unmarked learning stays non-shareable and is never enqueued — `platform/edge/host/tests/unit/mark_shareable.rs` (FR-011, B1).

### Implementation

- [ ] T020 [P] [US1] Hub `POST /runs` route: enrollment gate → schema-validate → upsert `run_metadata` on `run_id` in `platform/hub/src/routes/runs.ts` (FR-006, FR-010). Makes T015 pass.
- [ ] T021 [P] [US1] Hub `POST /learnings` route: enrollment + curation gate → upsert `learning` on `uid` in `platform/hub/src/routes/learnings.ts` (FR-007). Makes T016 pass.
- [ ] T022 [US1] Edge durable sync queue (atomic temp-then-rename writes; idempotency keys) in `platform/edge/host/src/sync/queue.rs` (FR-009, FR-010, R-3).
- [ ] T023 [US1] Edge hub client + backoff flusher (auth token; POST runs/learnings; metadata-only payload assembly) in `platform/edge/host/src/sync/client.rs` (FR-006/007/008/009, R-4). Depends on T022.
- [ ] T024 [US1] Hook run-completion + operator-initiated learning save (carried `save_memory`, curation gate) to enqueue sync in `platform/edge/host/src/orchestrator/` + ipc (FR-008, FR-011).
- [ ] T024a [US1] Implement the **"mark shareable"** transition — a `curate_learning` action (Rust) + IPC command + UI control that sets `curation_state=curated`; enforce the `{auto,captured,curated}` enum from `learning.schema.json` — in `platform/edge/host/src/` + `platform/edge/ui/` (FR-011, B1). Makes T019e pass.
- [ ] T025 [US1] Emit US1 observability — `wagner_sync_total{op,status}`, `wagner_sync_queue_depth`, `run.sync` span, plan §1.3 log fields — in `platform/edge/host/src/sync/` and hub routes.

**Checkpoint:** US1 Independent Test passes end-to-end. Shared, attributed, privacy-safe memory accumulates. MVP shippable.

---

## Phase 4 — User Story 2 — Start a run informed by what the org already knows (Priority: P2)

**Goal:** At run start in an enrolled project, surface org-wide learnings (across enrolled projects) relevant to the goal by tag + text, non-blocking and empty-on-unreachable.
**Independent Test:** With a learning in the hub from a prior run in a related area, start a new run on that area; verify the learning is surfaced at start, sourced org-wide across enrolled projects, and not surfaced for an unrelated goal.

### Tests (write FIRST, confirm FAILING)

- [ ] T026 [P] [US2] Contract test `GET /recall` — tag+text match, ≤10 recency-ordered, org-wide across enrolled projects, `[]` (not error) on no match — `platform/hub/tests/contract/recall.test.ts` (FR-012/013/014, US2-AS-1/2/4).
- [ ] T027 [P] [US2] Integration: relevant learning surfaced at run start; unrelated goal surfaces nothing — `platform/edge/host/tests/integration/recall.rs` (US2-AS-1/2, SC-003).
- [ ] T028 [P] [US2] Integration: hub unreachable → recall empty + run proceeds; unenrolled project → recall empty — `platform/edge/host/tests/integration/recall_degraded.rs` (FR-015, FR-016, US2-AS-3/5, Article VI).
- [ ] T028a [P] [US2] Test: a recall query emits `wagner_recall_total`, records `wagner_recall_latency_seconds`, and opens a `run.recall` span — `platform/edge/host/tests/integration/recall_observability.rs` (plan §1.3; challenge C3).
- [ ] T028b [P] [US2] Integration: run start surfaces **two labeled recall sets** — the carried local fold (own learnings, any state) + the hub `curated` org-wide block — and drops hub hits whose `uid` is already local (dedup) — `platform/edge/host/tests/integration/recall_two_source.rs` (FR-012, B-recall).

### Implementation

- [ ] T029 [P] [US2] Hub recall: **SurrealDB BM25** analyzer on learning text + tags filter; `GET /recall` query with `project_key` enrollment join, recency order, cap 10 — `platform/hub/src/recall/index.ts` + store (FR-013/014, R-2, ADR-0001). Makes T026 pass.
- [ ] T030 [US2] Edge recall client (query hub at run start) + surface results as a **distinct labeled hub block** alongside the carried local fold, dropping hub hits whose `uid` is already local (dedup) — `platform/edge/host/src/sync/recall.rs` + `platform/edge/ui/store/` surfacing (FR-012, US2, B-recall). Makes T028b pass.
- [ ] T031 [US2] Emit US2 observability — `wagner_recall_total`, `wagner_recall_latency_seconds`, `run.recall` span — per plan §1.3.

**Checkpoint:** US1 + US2 both work independently; recall proven. Wedge thesis (sync + recall + identity) demonstrated end-to-end.

---

## Final Phase — Polish & Cross-Cutting Concerns

- [ ] T032 [P] Author `platform/specs/001-shared-runs-and-learnings/quickstart.md` — enroll → run → sync → recall walkthrough.
- [ ] T032a [P] Performance test (write FIRST): recall p50 ≤ 2 s against a **SurrealDB** hub seeded with 5,000 learnings over a LAN/local connection — `platform/hub/tests/integration/recall_perf.test.ts` (SC-006; challenge C1).
- [ ] T033 Performance: make T032a pass — measure recall p50 at the SC-006 conditions and tune the **SurrealDB BM25 analyzer/index** if needed.
- [ ] T034 Security hardening: token handling, secrets from env only (D-SEC-2), schema validation at every route (Article X), privacy-boundary re-confirmation (SC-002).
- [ ] T035 [P] Docs: `platform/README.md` (run/test edge + hub); flip `platform/prd.md` Phase-1 status.
- [ ] T036 Refactor per /tdd Refactor phase; confirm dependency-direction (T007), replay (T006), privacy-boundary (T017), and enrollment-boundary (T019) tests stay green.

---

## Dependencies & Execution Order

- **Setup (1)** → no deps; first.
- **Foundational (2)** → depends on Setup; **blocks** all user-story phases. Within it: tests T005–T009b precede impl T006a + T010–T014b. **T006 (run-event replay test) precedes T006a (run spine build).**
- **US1 (3)** → depends on Foundational; tests T015–T019e precede impl T020–T025 (incl. T024a).
- **US2 (4)** → depends on Foundational (and on US1 having a sync path to populate the hub for a realistic recall test); tests T026–T028b precede impl T029–T031.
- **Polish (Final)** → depends on US1 (+ US2).

### Parallel Opportunities

- `[P]` tasks in a phase run concurrently (distinct files).
- Hub-side and edge-side tasks within a story largely parallelise (different packages) until the edge client needs the route (T023 after T020/T021 exist to test against, or against the stubbed hub per D-TEST-4).
- After Foundational, US1 and US2 can be split across agents/worktrees; US1 is the independently-mergeable MVP.

## Implementation Strategy

1. Setup + Foundational → foundation green (identity, enrollment, schemas, spine, guard).
2. US1 → run the Independent Test → MVP (shared attributed memory).
3. US2 → run the Independent Test → recall proven.
4. Polish → perf/security/docs/quickstart.
5. Hand to `/execute-plan`, which runs `/tdd` per task in order.

## Notes

- Mark `- [x]` only after acceptance is verified (test passes; for impl tasks, the covering test passes).
- Edge↔hub tests run against a stubbed/in-memory hub (D-TEST-4); no live cloud dependency.
- Stop at any Checkpoint to validate before continuing.
- Coverage: every FR-001..017 and every acceptance scenario (US1-AS-1..6, US2-AS-1..5) maps to ≥1 test task; SC-001..007 map to T017 / T017 / T027 / T018 / T006 / T032a / T019 respectively. EC-007 (revocation) → T009a/T014a; FR-010 atomicity → T019a; FR-011 edge gate → T019b; observability → T019c (sync) + T028a (recall); EC-001 (max learning size) → T019d. **Amend 2026-06-15:** FR-011 mark-shareable transition → T019e/T024a; B2 `project_key` (+EC-008 no-remote) → T009b/T014b; FR-012 two-source recall → T028b/T030; OIDC auth (FR-002) → T008/T013.

---

## Amendment — Carried-claim audit (2026-06-15, pre-`/execute-plan`)

Update 21 owed a carried-claim audit before execution. Done. Every load-bearing carried claim was checked against `apps/wagner` at file:line. Held up: B1 (`memory.rs:138` `curation_state` only ever `"auto"`), B2 (`commands.rs:385,404` `project_id` = local path), no carried learning schema, `MemoryRecord` shape, local recall fold (`commands.rs:467-479`), reducer **purity**. Found wrong/overstated:

- **F1 [MAJOR, engineer decision = build the spine].** Run metadata is **not** event-sourced in carried code: `state/store.rs` persists the `Run` aggregate as an atomic snapshot ("validate → temp → fsync → rename"); `run_loop.rs` mutates `run` and calls `state::save(&run)` each iteration (lines 114/128/139/190/221/269); `WagnerEvent`s are a transient UI projection (`run_loop.rs:53` `emit` "No-op in tests") carrying operative-level fields only — no run status/iteration/cost/halt. So SC-005/FR-003/Article VIII cannot be met by porting `reducer.ts`. **Resolution:** wedge **builds** an event-sourced run spine — T006 (replay test, RED) + **T006a** (run-event model + pure run reducer), with emission wired into the ported orchestrator (T024). T004 reduced to porting the carried **UI-projection** reducer only.
- **F2 [MODERATE, handle in T029].** `recall()` (`memory.rs:151-173`) ranks by `created_at DESC` + tag filter; the `wagner_en` BM25 index is **defined but unused**. So ADR-0001/R-2's "the edge already does this recall with BM25" overstates it — hub tag+goal-text BM25 (FR-013) is new behavior, and edge-local (tag+recency) vs hub-org (BM25) rank differently (acceptable; the two labeled sources in FR-012 make it visible). SurrealDB hub **decision stands**; only the rationale needs the correction noted here.
- **F3 [MODERATE, applied in T003/T004].** D-PROJ-3's `district→activity` collides with the existing distinct `activity` field. **Corrected mapping:** `operative→agent`, `faction→engine_class`, `district→stage`, `activity` unchanged, district value `oracle→planner`. (Carried vocab is already mixed: subtasks use `agent_id`, events use `operative_id`.)

Backups: `platform/.backups/{spec,plan,tasks}-pre-audit-2026-06-15.md`.

### Session progress (2026-06-15, post-audit)

**Done + verified:**
- **T001** ✅ — `platform/` scaffolded: npm workspace (`shared`, `edge/ui`; vitest 4 + tsc 5.8), Rust workspace (`edge/host`, edition 2021 / rustc 1.90 — note: plan says "1.87/edition 2024", carried reality is 1.90/2021), Deno hub (`deno.json`). `cargo check` ✓ and `npm install` (47 pkgs) ✓.
- **T006 + T006a** ✅ — the event-sourced run spine, TDD RED→GREEN: `shared/reducer/{run-events.ts,run-reducer.ts}` + `run-reducer.test.ts` (7/7 vitest green; strict tsc clean). Replaying the run-event log from empty == the incrementally-folded live snapshot, byte-identical (SC-005); halt reasons (FR-006); malformed-log guards; verified pure (Article VIII).
- **T003 (partial)** ✅ clean parts — `shared/schemas/run.schema.json` + `transmission.schema.json` ported verbatim (vocab-clean); `learning.schema.json` authored (B1: `curation_state ∈ {auto,captured,curated}`, 8 KiB cap, `additionalProperties:false`, `wagner-learning.v1`). All 3 compile under ajv draft-2020-12 + invariant checks pass.

**Deferred (next session), with reason:**
- **T003 `event.schema.json` (wagner-event)** — blocked on a vocab decision: D-PROJ-3 renames `faction→engine_class`, but the **enum values `architects`/`forgers`** have no platform mapping in the glossary. Need the engineer's call (e.g. keep values / `lead`+`worker` / `claude`+`codex`-class) before porting. Field renames are settled (F3): `operative_id→agent_id`, `district→stage`, value `oracle→planner`.
- **T004 (UI-projection reducer port)** — depends on the `event.schema.json` vocab above; it is the carried floor projection, not load-bearing for US1/US2 sync. Port after the value decision.
- **Rust-side run-event emission/fold parity** — the spine is canonical in TS (shared); when the orchestrator is ported (T024) the Rust host must emit + atomically append these events and a parity test should confirm its fold matches the TS reducer.
