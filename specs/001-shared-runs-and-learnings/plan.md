# Implementation Plan: Shared Coding Runs & Learnings (the platform wedge)

**Feature Branch:** `001-shared-runs-and-learnings`
**Date:** 2026-06-15
**Spec:** [spec.md](./spec.md)
**Constitution:** `platform/docs/spec/constitution.md` v0.1.0

This plan describes HOW the wedge is built. The WHAT lives in spec.md. Tasks (DO-THIS) live in tasks.md.

## Summary

The wedge ports the existing `apps/wagner` run engine into `platform/` and adds the smallest hub that proves the platform thesis: an edge operator runs a coding run locally (Rust/Tauri host + TS UI, subscription CLIs, event-sourced via the carried pure reducer), and — for **enrolled** projects under a **verified** identity — the run's metadata and operator-authored learnings sync to a small **Deno + Hono (TypeScript) + SurrealDB** hub that other operators' runs recall by tag + goal-text. Code and transcripts never cross the boundary; the run completes fully offline if the hub is unreachable, with sync queued and retried. The hub is deliberately small (a Deno/Hono service + a SurrealDB server, BM25 recall mirroring the edge analyzer, OIDC-SSO identity, an enrollment registry) and adopts no Temporal and no worker tier. (Hub stack per ADR-0001; identity per ADR-0002.)

---

## Technical Context

| Field | Value |
|-------|-------|
| Language / Version | **Edge host:** Rust 1.87 (Edition 2024), Tauri (carried from `apps/wagner`). **Edge UI:** TypeScript 5.x. **Hub:** TypeScript on **Deno** (ADR-0001). |
| Primary Dependencies | **Edge:** Tauri, the carried pure reducer (TS), serde/schema validation (Rust). **Hub:** `hono` (HTTP), `surrealdb` JS SDK (SurrealDB + BM25 full-text), `ajv` (JSON Schema draft 2020-12 validation), an OIDC client (Google/JumpCloud — ADR-0002). |
| Storage | **Edge:** local append-only event log + durable sync queue under the project's `.wagner/` (carried pattern); carried local SurrealDB memory store. **Hub:** SurrealDB (operators, projects/enrollments, run-metadata, learnings; BM25 analyzer on learning text, mirroring the edge's `wagner_en`). |
| Testing | `cargo test` (edge host, scripted `EngineRunner` — D-TEST-1); `vitest` (edge reducer/UI — D-TEST-2); `deno test` (hub logic); schema-validation tests (D-TEST-3); stubbed / in-memory SurrealDB hub for sync tests (D-TEST-4); `bats` (packaging — D-TEST-5). |
| Target Platform | **Edge:** macOS primary, Linux best-effort (carried). **Hub:** Linux — a Deno service + a SurrealDB server (ADR-0001). |
| Project Type | Multi-package within `platform/`: `shared/` (schemas + reducer), `edge/` (host + ui), `hub/` (service). |
| Performance Goals | Recall surfaced at run start p50 ≤ 2 s (SC-006, provisional); sync background/best-effort, never user-blocking. |
| Constraints | Edge offline-capable (Article VI); no metered LLM API key (Article VI, D-SEC-1); only metadata + learnings cross the boundary (Article IX); every boundary payload schema-validated (Article X); nothing outside `platform/` may import `platform/` (Article VII). |
| Scale / Scope | Wedge = single trusted org: tens of operators, low-thousands of runs/learnings. SurrealDB (single server, BM25 recall) covers this comfortably; no horizontal scaling in the wedge. |

> No remaining Technical-Context unknowns block the plan. Minor open items (recall result cap, max learning size, enrollment-revocation behaviour — spec CL-5/7/8) are resolved inline in Phase 0 / left as bounded plan defaults.

---

## Constitution Check

Gates refer to `platform/docs/spec/constitution.md` v0.1.0 (Articles I–X). Failed/added items have Complexity Tracking entries below.

- [x] **Gate I — Test-First:** tasks.md lists every test task before the implementation task it covers.
- [x] **Gate II — Evidence-Driven:** quantified or cited throughout; spec-challenger vague-adjective scan to confirm zero HIGH.
- [x] **Gate III — CRITICAL-Resolved:** no open CRITICAL findings at plan time.
- [x] **Gate IV — Independent MVP:** US1 has a concrete, non-empty Independent Test; shippable without US2.
- [⚠] **Gate V — Simplicity:** the wedge adds a hub service (Deno/Hono), **SurrealDB**, an **OIDC-SSO** auth surface, and an enrollment registry. Each has a Complexity Tracking entry below; all are irreducible given the spec (the thesis *is* edge→hub sync) or are explicit engineer decisions (CL-2; ADR-0001, ADR-0002).
- [x] **Gate VI — Edge-Executes-Hub-Remembers:** no hub round-trip is on the edge-run critical path (see §1.4 flow); recall is non-blocking and empty-on-unreachable; sync is post-completion/background; the run completes offline; no API key set. Offline-completion test planned (T018).
- [x] **Gate VII — One-Directional Dependency:** only `platform/` consumes the library; within `platform/`, `edge`/`hub` depend on `shared`, never the reverse, and nothing outside `platform/` imports it. Dependency-direction test planned.
- [x] **Gate VIII — Event-Sourced Truth:** log-replay-equals-snapshot test planned. **Audit 2026-06-15 (F1): the carried `reducer.ts` is a UI projection and the carried host snapshot-persists run state (not an event log) — so the event-sourced run spine is *wedge-built* (run-event model + pure run reducer; tasks T006/T006a), not a port. Engineer decision: build it.**
- [x] **Gate IX — Privacy-Boundary:** sync payload schema is metadata + learnings only (`additionalProperties: false`); default-sync-transmits-no-code test + enrollment-boundary test (SC-007) planned.
- [x] **Gate X — Schema-Validated:** every write/emit/sync validated; CI validates committed schemas + sample payloads.

---

## Project Structure

```
platform/
├── shared/
│   ├── schemas/                 # canonical JSON Schemas (draft 2020-12)
│   │   ├── run.schema.json           # carried wagner-run, renamed vocab (D-PROJ-3)
│   │   ├── event.schema.json         # carried wagner-event, renamed vocab
│   │   ├── transmission.schema.json  # carried
│   │   ├── learning.schema.json      # NEW: MemoryRecord shape carried; curation_state {auto,captured,curated} authored
│   │   ├── sync-run.schema.json      # NEW: edge→hub run-metadata payload (metadata only)
│   │   ├── sync-learning.schema.json # NEW: edge→hub learning payload
│   │   ├── recall-request.schema.json / recall-response.schema.json  # NEW
│   │   ├── enrollment.schema.json    # NEW
│   │   └── auth.schema.json          # NEW
│   └── reducer/                 # ported pure reducer (TS) + types; no I/O, no UI dep
│       ├── reducer.ts
│       └── reducer.test.ts
├── edge/
│   ├── host/                    # Rust (Tauri) host (ported from apps/wagner/src-tauri)
│   │   ├── src/
│   │   │   ├── orchestrator/    # goal loop over EngineRunner (carried)
│   │   │   ├── state/           # atomic, schema-validated local event log (carried)
│   │   │   ├── sync/            # NEW: durable sync queue + hub client + retry
│   │   │   └── ipc/             # Tauri commands (carried + recall/enroll/auth)
│   │   └── tests/{unit,integration}/
│   └── ui/                      # TS frontend (ported store/bridge; recall surfacing)
│       ├── store/               # imports shared/reducer
│       └── *.test.ts
└── hub/                         # NEW — Deno + Hono (TS) + SurrealDB service
    ├── src/
    │   ├── routes/              # /auth /projects /runs /learnings /recall
    │   ├── store/               # SurrealDB access (operators, projects, runs, learnings)
    │   ├── recall/              # BM25 tag+text query (mirrors the edge analyzer)
    │   └── validate/            # ajv against shared/schemas
    └── tests/{contract,integration,unit}/
```

**Structure Decision:** Three packages under one `platform/` tree keep the one-directional dependency explicit (`edge`,`hub` → `shared`; never the reverse — Gate VII) and let `shared` hold the carried spine (schemas + pure reducer) that both layers validate against (Gate VIII, X). It is the smallest layout that separates the two runtime layers the PRD mandates without inventing extra projects (Gate V). The edge host stays Rust/Tauri (D-PROJ-4, no rewrite); the hub is a Deno/Hono service talking to a SurrealDB server — two cooperating processes, no broker, no queue, no Temporal — to stay within "small hub" (ADR-0001).

---

## Phase 0 — Research

Inlined (Decision / Rationale / Alternatives). A standalone `research.md` can be extracted on request.

- **R-1 — Verified identity mechanism (FR-002, spec CL-2). [SUPERSEDED by ADR-0002.]**
  - **Decision:** **OIDC SSO** against the org IdPs (Google and JumpCloud). The hub stores **no** credentials; it validates an IdP-issued ID token (issuer / audience / signature) and gates access to verified employees by email domain / IdP group; `operator_id` = the IdP subject. The Tauri edge uses Authorization-Code + PKCE; auth is required only at the sync/recall boundary, so edge autonomy holds (Article VI).
  - **Rationale:** The platform is employees-only, so the access gate must be real; OIDC outsources credential handling to the IdP (a smaller, safer hub surface than a hand-rolled secret store); auth is hard to retrofit once learnings are attributed. Both org IdPs speak OIDC, so one client covers both.
  - **Alternatives:** hand-rolled email + secret + bearer (the prior R-1 pick — makes the hub a credential authority, has no employee gate, forces an identity migration later); SAML (worse fit for a native/PKCE client); device-key / mTLS (proves device, not person, identity).

- **R-2 — Tag + goal-text recall in SurrealDB (FR-013, CL-4). [SUPERSEDED by ADR-0001.]**
  - **Decision:** SurrealDB **BM25** full-text analyzer over learning text (mirroring the carried edge `wagner_en` analyzer so ranking matches edge↔hub), plus a tags filter and a `project_key` enrollment join; rank by recency among matches, cap at 10 (provisional, CL-5).
  - **Rationale:** The carried edge already runs SurrealDB + BM25 doing exactly this recall (`memory.rs:118-120,151-173`); reusing the engine keeps one model and identical ranking. No embedding/vector index — meets the Phase-3 deferral of semantic recall.
  - **Alternatives:** SQLite FTS5 (the prior R-2 pick — a second engine + recall-ranking divergence from the edge); `LIKE` scans (no ranking); external search engine (overkill for low-thousands of rows).

- **R-3 — Durable edge sync queue + retry (FR-009, Article VI).**
  - **Decision:** A local durable queue (a small SQLite table or append-only JSONL under `.wagner/`), written atomically (temp-then-rename, D-RES-1); a flusher drains it with exponential backoff when the hub is reachable; entries keyed by run_id / learning uid for idempotent upsert.
  - **Rationale:** Survives process restart and offline stretches; idempotency keys make retries safe (no duplicate hub records).
  - **Alternatives:** In-memory queue (loses queued syncs on crash — fails Article VI durability).

- **R-4 — Edge↔hub transport.**
  - **Decision:** HTTP/JSON (REST) against the Hono API; every request/response body validated against `shared/schemas` on both ends (Article X).
  - **Rationale:** Simplest cross-language contract (Rust client → TS server); human-inspectable for the privacy-boundary test (SC-002).
  - **Alternatives:** gRPC (codegen + tooling weight not justified at wedge scale).

---

## Phase 1 — Design & Contracts

### 1.1 Data Model (hub-side, SurrealDB)

- **operator:** `id` (= IdP subject), `email`, `created_at`. The verified identity via OIDC (ADR-0002) — **no credential/secret stored**. One per person.
- **project_enrollment:** `id`, `operator_id` (FK), `project_key`, `enrolled_at`. `project_key` = normalized git origin remote (B2). Presence gates sync + recall (FR-016/017). A project_key is enrolled or it is invisible to the hub in both directions.
- **run_metadata:** `run_id` (PK, idempotency key), `operator_id` (FK), `project_key`, `goal`, `status`, `halt_reason`, `iterations_used`, `cost_used`, `started_at`, `ended_at`. Metadata only — no code/transcript fields exist in this table (enforces Article IX at the schema level).
- **learning:** `uid` (PK, idempotency key), `operator_id` (FK), `project_key`, `text`, `tags` (normalized), `created_at`, `curation_state ∈ {auto, captured, curated}` (wedge-introduced enum; the carried field only ever held `"auto"`). Synced only when `curation_state = curated` (D-DATA-3); `auto` and `captured` learnings stay local and are never enqueued (FR-011). `text` capped at 8 KiB — the `sync-learning` schema enforces `maxLength` and an oversize save is rejected, not truncated (EC-001/CL-7). A **SurrealDB BM25 analyzer** indexes `text` (mirrors the edge `wagner_en`).

Validation rules trace to FRs: every row carries `operator_id` (FR-001) and a `project_key` that MUST match an enrollment row (FR-016); inserts are upserts on the PK (R-3 idempotency).

### 1.2 Interface Contracts (Hono HTTP API; all bodies schema-validated, Article X)

- **POST `/sessions`** (OIDC) → exchange a validated IdP ID token (Google/JumpCloud) for a hub session; the hub verifies issuer/audience/signature + employee domain/group and upserts the operator by IdP subject (ADR-0002, R-1). Error model: 401 on invalid/expired token; 403 on non-employee. `auth.schema.json`.
- **POST `/projects/enroll`** (auth required) → enroll `project_key` for the operator. Idempotent (re-enroll is a no-op). 200. `enrollment.schema.json`.
- **GET `/projects`** (auth) → list the operator's enrolled projects (FR-017 inspectable).
- **POST `/runs`** (auth) → upsert run metadata; 422 if `project_key` not enrolled (FR-016); body validated against `sync-run.schema.json` (metadata only; extra fields rejected by `additionalProperties:false` — SC-002).
- **POST `/learnings`** (auth) → upsert a learning; 422 if not enrolled or `curation_state` not shareable; `sync-learning.schema.json`.
- **GET `/recall?goal=…&tags=…&project_key=…`** (auth) → returns ≤10 learnings across all enrolled projects org-wide (FR-014), tag+text matched (FR-013), recency-ordered; `recall-response.schema.json`. Returns `[]` (never an error) when nothing matches.

Idempotency: `/runs` and `/learnings` upsert on `run_id`/`uid`, so the edge retry queue (R-3) cannot create duplicates. Versioning: schemas carry a `schema` const (carried convention, D-STORE-1).

### 1.3 Cross-Cutting Concerns

#### Observability
- **Log fields (hub + edge sync):** `operator_id`, `project_key`, `run_id`/`learning_uid`, `op` (`sync_run`|`sync_learning`|`recall`|`enroll`|`auth`), `outcome`, `duration_ms`, `queued` (bool), `schema_valid` (bool).
- **Metrics (hub):** `wagner_sync_total{op,status}`, `wagner_recall_total`, `wagner_recall_latency_seconds` (histogram), `wagner_enrolled_projects`. **Edge:** `wagner_sync_queue_depth`.
- **Trace spans:** `run.recall` (at start), `run.sync` (post-completion), each child of the run.

#### Security
- **Trust boundary:** the edge→hub HTTP boundary. Every inbound body is validated against `shared/schemas` before any DB write (Article X); unknown fields are rejected (`additionalProperties:false`).
- **Authentication:** OIDC — the hub validates the IdP-issued ID token (issuer/audience/signature) and gates by employee domain/group, then issues a short-lived hub session (ADR-0002, R-1, FR-002).
- **Authorisation:** an operator may write only records attributed to their own `operator_id`; sync/recall are refused for non-enrolled `project_key`s (FR-016). Recall reads are org-wide across enrolled projects (FR-014, CL-2).
- **Secrets:** hub session-signing secret, OIDC client config, and the SurrealDB connection from environment, never inlined (D-SEC-2). Edge sets no LLM API key (D-SEC-1).
- **Privacy:** no schema in `shared/schemas` for a sync payload contains a code/diff/transcript field — the boundary is enforced structurally, not by convention (Article IX; SC-002).

#### Failure Modes
- **Retries:** the edge sync queue retries with exponential backoff; upserts are idempotent (R-3).
- **Graceful degradation:** hub unreachable → run completes locally, sync queues, recall returns empty (Article VI; D-RES-2; SC-004).
- **Data integrity:** atomic local writes (D-RES-1); eventual consistency on the hub via idempotent upserts; a partial sync (metadata landed, learning didn't) reconciles on the next flush (EC-003).

### 1.4 Edge-run critical path (Gate VI proof)

The edge run's critical path touches the hub **zero times synchronously**. Hub interactions are either pre-flight-and-optional (recall) or post-terminal-and-queued (sync):

```
run start
  └─ recall query (async, time-boxed) ──► hub        [OPTIONAL: on timeout/unreachable → [] , run continues]
  └─ goal loop executes locally on subscription CLIs  [CRITICAL PATH — no hub call here]
       └─ events fold into local append-only log (pure reducer)
  └─ run reaches terminal state (met/halted/aborted)  [run is DONE here, hub or no hub]
       └─ enqueue metadata + curated learnings to the LOCAL durable queue (atomic)
            └─ flusher drains queue ──► hub            [BACKGROUND: retried w/ backoff; off critical path]
```

Recall (US2) is a best-effort, time-boxed read before work begins; its failure yields `[]` (FR-015), never a block. Sync (US1) is enqueued *after* the run is already terminal, so a slow/absent hub cannot delay completion (FR-009). This is the architectural form of Article VI; the offline-completion test (T018) and recall-degraded test (T028) assert it.

---

## Complexity Tracking

| Violated Gate | Why Needed | Simpler Alternative Rejected Because |
|---------------|------------|--------------------------------------|
| Gate V — new **hub service** (TS/Hono) | The wedge's entire thesis is edge→hub sync + shared recall + identity; a hub is the irreducible minimum to prove it (`platform/prd.md` §"the hub remembers"). | No-hub / file-share: cannot provide cross-operator recall or verified identity at all. |
| Gate V — **SurrealDB** datastore (ADR-0001) | Durable storage for operators/enrollments/runs/learnings + tag/text recall (BM25); the carried edge already runs SurrealDB+BM25, so the engine + recall are reused, not reinvented. | SQLite/FTS5: a second engine + recall-ranking divergence from the edge. In-memory: loses shared memory across restarts. |
| Gate V — **OIDC-SSO auth surface** (FR-002, ADR-0002) | Engineer requires an employees-only gate; OIDC verifies identity against the org IdPs and stores no credentials in the hub. | Self-asserted identity: rejected. Hand-rolled secret store: makes the hub a credential authority with no employee gate. |
| Gate V — **enrollment registry** (FR-017) | Engineer requires per-project opt-in so personal/unenrolled projects never enter shared memory (CL-2). | Sync-everything: leaks personal-project learnings; violates the privacy intent (Article IX). |

All four additions are bounded to the hub and traceable to either the irreducible thesis or a recorded engineer decision (ADR-0001, ADR-0002); none introduce a queue broker or Temporal. The SurrealDB server (one process) and the OIDC IdP (Google/JumpCloud, already operated by the org) are accepted dependencies — the Gate V intent (no broker, no queue, no Temporal, no managed datastore to run ourselves) is preserved.

---

## Optional Artifacts

Created on request, not by default:

- [ ] `data-model.md` — full SurrealDB schema, indices, BM25 analyzer config (summarized inline §1.1).
- [ ] `contracts/*.yaml` — OpenAPI for the Hono API (summarized inline §1.2).
- [ ] `research.md` — Phase 0 findings (inlined above).
- [ ] `quickstart.md` — end-to-end wedge validation walkthrough.
