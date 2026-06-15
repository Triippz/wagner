# Change Set: Shared Coding Runs & Learnings (the platform wedge)

**Spec dir:** `platform/specs/001-shared-runs-and-learnings/`
**Engineer's change description:**

> Apply the `/interrogate-with-docs` decisions (2026-06-15): B1 curation_state→{auto,captured,curated} net-new + a "mark shareable" transition; B2 project_key=normalized git origin remote w/ boundary derivation; B3+B3b hub stack→Deno+Hono+SurrealDB (supersede plan R-2); B-auth identity→OIDC SSO Google/JumpCloud (override plan R-1, refine FR-002); B-recall→two labeled recall sources (local fold + hub curated block + uid dedup); fix C5 schema-port wording (learning.schema.json authored not ported; confirm/drop oracle-plan). See `platform/docs/adr/0001` and `0002`.

**Generated:** 2026-06-15
**Generator:** spec-driven-development skill (amend mode), driven manually against the platform tree (governance is platform-scoped).
**Source spec hash (before changes):** `29ca7337bc8f61939e5d4b2a8ff5ccc8952aae1c795d241e6a1d2debfac366bb`

> **Engineer-approval gate.** The substance of every row below was approved row-by-row during the `/interrogate-with-docs` session (B1, B2, B3, B3b, B-auth, B-recall) and the engineer said "Please run it." Rows are therefore pre-marked **ACCEPTED**. Two ADRs (`0001`, `0002`) already record the two hardest reversals.
>
> **Apply deviation (deliberate):** this repo's git policy is "commit only when asked," and the entire `platform/` tree is intentionally uncommitted. So `--apply` writes to the **working tree** and snapshots the prior artifacts to `platform/.backups/` — it does **not** make one-commit-per-row. The engineer commits the platform tree when ready.

---

## Parsed Changes

| Clause (from description) | Verb match | Action |
|---|---|---|
| curation_state → {auto,captured,curated} net-new | `new` | ADD/MODIFY |
| add a "mark shareable" transition | `add` | ADD |
| project_key = normalized git origin remote | `new` (implicit) | ADD/MODIFY |
| hub stack → Deno+Hono+SurrealDB; supersede R-2 | `replace`/`supersede` | MODIFY |
| identity → OIDC SSO; override R-1, refine FR-002 | `override`/`refine` | MODIFY |
| two labeled recall sources | `add` | ADD/MODIFY |
| fix schema-port wording; confirm/drop oracle-plan | `update`/`drop` | MODIFY/REMOVE |

No `[UNPARSED]` clauses.

---

## Proposed Changes

Grouped by decision (each group = one logical change with the exact sub-locations it touches). Confidence HIGH unless noted — every target is an explicit ID or a uniquely-titled section in the current artifacts.

### B1 — Curation lifecycle is net-new, not carried (+ "mark shareable")

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C1 | MODIFY | spec §Key Entities (Learning) | `curation_state ∈ {captured, curated}` → `∈ {auto, captured, curated}`; relabel **"introduced by the wedge,"** not "Carried." `auto`=machine-suggested/local, `captured`=operator-authored/local, `curated`=operator-marked-shareable (only state that syncs). **Evidence:** carried `memory.rs:138,241,261` only ever writes `"auto"`; `bridge.ts:170` types it open `string`; no `captured`/`curated`/transition exists. | ACCEPTED (interrogation B1) |
| C2 | MODIFY | spec §FR-007 | Clarify the gate: `auto` AND `captured` stay local; **only `curated` syncs**. | ACCEPTED (B1) |
| C3 | MODIFY | spec §FR-011 | Enum is `{auto,captured,curated}` introduced by the wedge; edge MUST NOT enqueue while `auto` **or** `captured` (was: only `captured`); add an explicit operator **"mark shareable"** action that sets `curated`. | ACCEPTED (B1) |
| C4 | MODIFY | spec §Defaults Applied (D-DATA-3) | Note the `{auto,captured,curated}` enum + the curate transition are wedge-introduced, not carried. | ACCEPTED (B1) |
| C5 | ADD | tasks §Phase 2 Foundational | New RED test + impl for the **"mark shareable" transition** (Rust transition + IPC command + UI) and the closed enum in `learning.schema.json`. Broaden T019b: `auto` AND `captured` are not enqueued; only `curated` is. | ACCEPTED (B1) |

### B2 — `project_key` = normalized git origin remote

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C6 | MODIFY | spec §Key Entities (Project enrollment) + FR-016/FR-017 | Introduce **`project_key` = normalized git origin remote** as the shared/enrollment identity; the carried local `project_id` (a filesystem path — `commands.rs:385`) is edge-only; enrollment + recall key on `project_key`, derived at the sync/enroll boundary. **Evidence:** carried `project_id` is the local path → differs per operator → SC-003 cross-person recall can't hold without a stable shared key. | ACCEPTED (B2) |
| C7 | ADD | spec §Edge Cases (EC-008) | A project with **no git remote** cannot be enrolled → runs fully local (no sync, no recall), exactly as if no hub existed. Note: a monorepo behind one remote collapses to one `project_key` (accepted wedge limitation). | ACCEPTED (B2) |
| C8 | MODIFY | spec §SC-003 | "any operator in any enrolled project" is keyed on `project_key` (shared remote identity). | ACCEPTED (B2) |
| C9 | ADD/MODIFY | tasks §Phase 2 Foundational | RED test: two different local paths with the same git origin remote yield one `project_key`; no-remote ⇒ no enrollment. + impl: derive `project_key` at the sync/enroll boundary. | ACCEPTED (B2) |

### B3 + B3b — Hub stack → Deno + Hono + SurrealDB (ADR-0001; supersede R-2)

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C10 | MODIFY | plan §Summary + §Technical Context | Hub = **Deno + Hono (TS) + SurrealDB** via JS SDK (not Node + SQLite/FTS5 + better-sqlite3). Storage = SurrealDB (operators/enrollments/runs/learnings; BM25 analyzer mirroring the edge's `wagner_en`). `ajv` retained for Article X JSON-Schema validation. | ACCEPTED (B3/B3b, ADR-0001) |
| C11 | MODIFY | plan §Phase 0 R-2 | Superseded: recall = SurrealDB BM25 analyzer + tags + enrollment join, recency, cap 10 — mirrors the carried edge analyzer (not SQLite FTS5). **Evidence:** carried edge already runs SurrealDB+BM25 (`memory.rs:17,118-120,151-173`). | ACCEPTED (B3, ADR-0001) |
| C12 | MODIFY | plan §1.1 Data Model + §1.2 Contracts + §Project Structure | SurrealDB tables (not SQLite); `project_key` on enrollment/run/learning; `curation_state ∈ {auto,captured,curated}`; hub tree reflects SurrealDB store + recall. | ACCEPTED (B3) |
| C13 | MODIFY | plan §Complexity Tracking + §Constitution Check | SQLite → SurrealDB (ref ADR-0001); keep "small hub" (two cooperating processes: Deno + SurrealDB; no broker/queue/Temporal). | ACCEPTED (B3) |
| C14 | MODIFY | tasks T002, T012, T029, T032a, T033 | T002 hub deps → `hono`, `surrealdb`, `ajv` (drop `better-sqlite3`); T012 init SurrealDB store; T029/T032a/T033 recall + perf via SurrealDB BM25 (not FTS5). | ACCEPTED (B3) |

### B-auth — Identity = OIDC SSO Google/JumpCloud (ADR-0002; override R-1, refine FR-002)

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C15 | MODIFY | spec §FR-002 + §Key Entities (Operator identity) | Mechanism = **OIDC SSO** against org IdPs (Google, JumpCloud), employees-only gate; hub validates IdP-issued ID token (issuer/aud/signature) + domain/group; `operator_id` = IdP subject. (FR-002 already permitted "token / SSO / device key".) | ACCEPTED (B-auth, ADR-0002) |
| C16 | MODIFY | plan §Phase 0 R-1 + §1.2 + §1.3 Security + §Complexity Tracking | Superseded: OIDC (Auth-Code + PKCE on the Tauri edge); hub stores **no** credentials; auth only at the sync/recall boundary (edge autonomy preserved). | ACCEPTED (B-auth, ADR-0002) |
| C17 | MODIFY | tasks T008, T013 | Auth → OIDC: RED contract test (valid ID token accepted; bad/expired/wrong-aud and non-employee domain rejected) + impl OIDC validation middleware + edge PKCE flow (replaces email+secret register/session). | ACCEPTED (B-auth) |
| C18 | MODIFY | spec §Dependencies | Add IdP (Google/JumpCloud OIDC) dependency + failure mode (IdP unreachable → can't (re)auth → sync queues, run still completes); hub line → Deno + SurrealDB. | ACCEPTED (B-auth/B3) |

### B-recall — Two labeled recall sources

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C19 | MODIFY | spec §US2 + §FR-012 | Recall surfaces **two labeled sources**: the carried local fold (operator's own, this project, any curation state) AND the hub's org-wide `curated` set; drop hub hits whose `uid` is already in the local set (1-line dedup). **Evidence:** carried local fold ships at `commands.rs:467-479`. | ACCEPTED (B-recall) |
| C20 | ADD | tasks §Phase 4 US2 | RED test + impl: two-source recall — carried local fold retained, hub curated block added, `uid` dedup between them. | ACCEPTED (B-recall) |

### C5-cleanup — Schema-port honesty

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C21 | MODIFY | tasks T003 | `learning.schema.json` is **authored** (NEW — no such file exists in `apps/wagner/schemas/`), not "ported." Port `{run,event,transmission}`. **Decide oracle-plan:** either port as `planner.schema.json` or record the planner schema out of wedge scope (recall feeds "the planning step" but the wedge needs no planner schema). | ACCEPTED (C5) — planner: **out of wedge scope** (recorded) |
| C22 | MODIFY | checklists/requirements.md | CHK032: `curation_state` was NOT defined-as-carried — now `{auto,captured,curated}` introduced by wedge. CHK005/CHK007: hub tech (SurrealDB/Deno) + identity (OIDC) now decided. Add note: C1–C5 carried-claim corrections applied 2026-06-15. | ACCEPTED (C5) |

### Audit — record the reopened findings

| ID | Action | Target | Proposed content | Disposition |
|----|--------|--------|------------------|-------------|
| C23 | ADD | spec §Clarifications | New block "Session 2026-06-15 (interrogation amendments)": record B1/B2/B3/B3b/B-auth/B-recall; note challenge **H4 was resolved on a false "carried" premise** (the `{captured,curated}` enum was not carried — `memory.rs` writes only `"auto"`); list code contradictions C1–C5 from the interrogation. | ACCEPTED |

---

## Unresolved References

| Source change | Referenced from | Cascade |
|---|---|---|
| C10/C11/C14 (SQLite → SurrealDB) | validation-report.md (READY rests partly on SQLite mappings + a false "learning schema confirmed present" at :173) | validation-report.md is **regenerated** by `/spec validate` after apply (Phase 6) — no manual cascade |
| C15/C16/C17 (secret → OIDC) | plan R-1, T008/T013 | covered by C16/C17 |
| C1/C3 (enum) | challenge H4 "RESOLVED" | covered by C23 (recorded as resolved-on-false-premise; now genuinely resolved) |
| All | `apps/wagner` carried code | NOT modified — the wedge ports/introduces; carried source is untouched until `/execute-plan` |

---

## Validator Pre-Check

| Metric | Before | After (predicted) |
|--------|--------|-------------------|
| FR/SC/AS coverage | 17/17, 7/7, 11/11 | maintained + new FRs/tasks (curate transition, project_key, OIDC, two-source recall) each added tests-first |
| Open CRITICAL | 0 | 0 (all additions are Article-I tests-first; no FR left without a task) |
| Constitution gates | 10/10 | 10/10 (Gate V Complexity Tracking updated for SurrealDB + OIDC; Gate VI preserved — auth only at sync/recall boundary) |
| Verdict | READY | **READY (pending re-validation)** — must re-run `/spec validate` to confirm coverage of the new tasks |

No predicted regression. Re-validation (Phase 6) is required because new behaviour-changing tasks were added.

---

## Engineer Acknowledgement

Substance approved during `/interrogate-with-docs` (2026-06-15); rows pre-marked ACCEPTED. Apply writes to the working tree + `platform/.backups/` (no per-row commits, per repo git policy).
