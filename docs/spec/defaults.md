# Wagner Platform Defaults Catalog

**Purpose:** This file is the **only** place the spec-driven workflow is allowed to apply
platform-wide defaults. If a piece of information is required by the spec template and is not
present in either (a) the engineer's input or (b) this catalog, the agent MUST mark it
`[NEEDS CLARIFICATION: not in defaults catalog]` rather than choose a value.

**Why:** Per project policy, the engineer fills gaps, not the agent. Silent agent assumptions
are forbidden. This catalog makes every default visible, version-controlled, and reviewable.
Adding a new default is an explicit edit to this file.

**Created:** 2026-06-14
**Last amended:** 2026-06-14
**Maintainer:** Mark Tripoli (mtripoli@adyton.io)

**Scope note:** This catalog governs the **platform runtime** (`platform/`) only. The
capability library has its own catalog at `docs/spec/defaults.md`. Items below marked with a
`[bracketed placeholder]` are deliberately *not yet decided* — the specifier MUST treat them
as `[NEEDS CLARIFICATION]`, not apply them.

---

## How to Use

- The agent reads this file at the start of every Specify, Import, or Amend phase.
- Every default the agent applies MUST be cited by section + bullet ID (e.g., `D-DATA-1`).
- Adding, modifying, or removing a default is an engineer-authored edit to this file. The
  agent MUST NOT extend the catalog itself.
- If a default applies in some contexts but not others, write the context as part of the
  bullet (e.g., "edge runs only").
- When the catalog and the engineer's input conflict, the engineer's input wins; the agent
  records the conflict in the spec under a `## Defaults Overridden` section.

---

## Authentication & Authorisation

- **D-IDENT-1**: Every run, event, and learning MUST carry an **operator identity** used for
  attribution and recall scoping (constitution Article IX). Identity is required on the
  edge→hub sync boundary.
- **D-IDENT-2**: [Default mechanism by which an operator authenticates to the hub — token,
  device key, SSO. Not yet decided.]

## Storage & Persistence

- **D-STORE-1**: All persistent state files use JSON with a declared schema-version field
  (e.g. `"schema": "wagner-run.v1"`). No YAML for state files. (Carried from `apps/wagner`
  and the library catalog convention.)
- **D-STORE-2**: Edge run state persists locally as an **append-only event log**; the log is
  the source of truth and run snapshots are projections of it (constitution Article VIII).
- **D-STORE-3**: Learnings persist as a `MemoryRecord` — `{uid, project_id, text, tags,
  created_at, curation_state}` — carried from `apps/wagner` `save_memory`/`recall_memory`.
- **D-STORE-4**: Edge state and learnings persist under the operator's project (the
  `apps/wagner` pattern writes to a project-local `.wagner/` directory); they are local
  artifacts, not hub-owned.
- **D-STORE-5**: [Default hub datastore for the shared event store + recall index. Not yet
  decided; the wedge requires only a *minimal* hub and MUST NOT adopt a managed datastore it
  does not need — see constitution Article V.]

## Performance & Scale

- **D-PERF-1**: A run MUST surface a liveness cue during a message-sparse turn so it never
  looks frozen (carried product invariant — `apps/wagner/PRODUCT.md` principle 4). [Exact
  staleness threshold in seconds: not yet decided.]
- **D-PERF-2**: [Default edge→hub sync latency / batching target. Not yet decided.]
- **D-PERF-3**: [Default hub recall query latency target. Not yet decided.]

## Observability

- **D-OBS-1**: Every run emits a normalized, append-only **event stream**; observation,
  steering, and recall all read from it (the run/event spine). Event payloads validate
  against `wagner-event` schema (Article X).
- **D-OBS-2**: [Default structured-log fields for the edge harness and hub. Not yet decided.]
- **D-OBS-3**: [Default metrics / alert thresholds for the hub. Not yet decided — Phase 2+.]

## Security

- **D-SEC-1**: Edge runs MUST set **no metered LLM API key** and drive only the operator's
  authenticated subscription `claude`/`codex` CLIs (constitution Article VI). The installer
  warns if an API-key env var would override the subscription (carried from `apps/wagner`).
- **D-SEC-2**: Secrets, tokens, and credentials MUST NOT be inlined in code or state files;
  they are loaded from the environment or a secret store at runtime.
- **D-SEC-3**: All external inputs — CLI output streams, hub payloads, recall responses —
  MUST be validated against a declared schema before use (constitution Article X).

## Data Handling

- **D-DATA-1**: By default, only run **metadata** (goal, status, phase, outcome, guardrail
  usage, timing) and curated **learnings** sync to the hub. The codebase, file diffs, and full
  agent transcripts stay on the edge (constitution Article IX).
- **D-DATA-2**: Sharing anything beyond metadata + learnings is an **explicit, per-item**
  operator action — never a default and never a single global toggle (constitution Article IX).
- **D-DATA-3**: A learning syncs only once it reaches a shareable `curation_state`; raw,
  uncurated capture is not assumed shareable (carried from `MemoryRecord.curation_state`).
- **D-DATA-4**: [Default retention for synced run metadata + learnings in the hub. Not yet
  decided.]

## Testing

- **D-TEST-1**: The edge host (Rust) is tested with `cargo test`. The goal loop runs over an
  `EngineRunner` trait so the full loop is verified without burning a subscription — tests
  script the runner; they never spawn a real CLI (carried from `apps/wagner`).
- **D-TEST-2**: Frontend / TypeScript logic is tested with `vitest`. The run reducer is pure
  and unit-tested in isolation, with no UI-framework or I/O dependency (constitution
  Article VIII).
- **D-TEST-3**: Schema-validation tests run against every committed schema file and a
  representative payload for it (constitution Article X).
- **D-TEST-4**: Edge↔hub interactions are tested against a **stubbed / in-memory hub** — no
  live cloud dependency in the test suite (mirrors the `EngineRunner`-trait discipline).
- **D-TEST-5**: Installer / packaging is tested with bats-core (carried from `apps/wagner`
  `tests/install/*.bats` and the repo's 159-test bats suite).

## Error Handling & Resilience

- **D-RES-1**: All state writes are atomic — write to a temp file, validate, rename. Partial
  writes MUST NOT be visible to consumers (carried from the library catalog and `apps/wagner`).
- **D-RES-2**: Edge runs degrade gracefully when the hub is unreachable: recall returns empty,
  sync queues locally and retries later, and the run is never blocked on the hub (constitution
  Article VI).
- **D-RES-3**: The reducer applies a **stale-event guard** — an older event never clobbers a
  newer one for the same actor (carried from `apps/wagner` `reducer.ts` `applyEvent`).

## Internationalisation & Accessibility

- **D-A11Y-1**: Operator-facing surfaces target **WCAG 2.1 AA**. State is never conveyed by
  color alone — every state carries a non-color glyph or text label; every animation has a
  `prefers-reduced-motion` alternative (carried from `apps/wagner/PRODUCT.md` and `DESIGN.md`).
- **D-I18N-1**: [Default supported locales for operator-facing surfaces. Not yet decided;
  current surfaces are en-US.]

## Project-Specific

- **D-PROJ-1**: The platform **consumes** the capability library and never adds a dependency
  from the library (`skills/`, `agents/`, `commands/`, `hooks/`, `plugins/`, `profiles/`) into
  `platform/` (constitution Article VII).
- **D-PROJ-2**: A new capability is modeled as a **new kind of `run`** over the shared
  run/event/transmission spine, not as a new subsystem (`platform/prd.md` §"the run primitive").
- **D-PROJ-3**: Floor-era vocabulary is renamed to platform vocabulary in all new platform
  code and specs: operative → **agent**, faction (`architects`/`forgers`) → **engine class**,
  district → **activity/stage**, oracle → **planner**. No floor terms appear in new platform
  surfaces (`platform/prd.md` rename table).
- **D-PROJ-4**: The edge stack ports from `apps/wagner` — **Rust host + TypeScript frontend**
  (Tauri). New edge work continues that stack unless the spec records a Complexity Tracking
  entry for a different one.
- **D-PROJ-5**: The wedge MUST NOT adopt **Temporal** or an always-on worker tier; those are
  Phase-2 (`platform/prd.md` §"Phases 2–3"). Edge runs are event-sourced on the local log.
- **D-PROJ-6**: [Default hub language / deploy target. Not yet decided — the wedge needs only
  a minimal hub; the choice is a Complexity-Tracking decision when the wedge plan is written.]

---

## Removed / Deprecated Defaults

When a default is removed, move it here with a `**Reason:**` line. Do not delete — the
historical record matters when reviewing old specs.

- *(empty)*

---

## Catalog Discipline

- The agent MUST cite the bullet ID (e.g., `D-DATA-1`) in `spec.md §Assumptions` whenever it
  applies a default from this catalog.
- The agent MUST NOT silently apply a default that is not in this catalog.
- When the catalog itself contains placeholders (`[…]` bracketed text), the agent MUST treat
  the placeholder as a `[NEEDS CLARIFICATION]` and flag the spec accordingly.
