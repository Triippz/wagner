# Implementation Plan: Event Bus Contracts (Phase 0)

**Feature Branch:** `013-event-bus-contracts`
**Date:** 2026-06-18
**Spec:** `specs/013-event-bus-contracts/spec.md`
**Constitution:** `docs/spec/constitution.md` v0.1.0

This plan describes HOW the contracts are built. The WHAT lives in `spec.md`. Tasks live in `tasks.md` (Phase 4, next).

## Summary

Define the public contract types of the event bus — `Envelope`, the namespaced `Event`/`Command` taxonomy (structure only; leaves deferred to `011` P0), `ParticipantId`/`Scope`/`StreamId`, the `Agent` trait signature, the uniform `PluginManifest`, the closed `Capability` vocabulary v1, and `StabilityTier` — as plain serializable Rust types in a new `edge/host/src/bus/` module of the existing `wagner-edge-host` crate. The single source of truth is the Rust type: `schemars` derives a JSON Schema (draft 2020-12, `additionalProperties:false`) per contract type, exported to `edge/host/schemas/`; `json-schema-to-typescript` generates TS bindings from that exported catalog into `shared/contracts/`. No bus loop, intake, registry loading, or participant behaviour is built — those are `011` P1+. The phase ships when serde round-trip, schema accept/reject, the no-handle seam guard, the additive-versioning regression, and the TS-binding-compiles tests all pass.

---

## Technical Context

| Field | Value |
|-------|-------|
| Language / Version | Rust, Edition 2021 (`[workspace.package] edition = "2021"`); toolchain 1.91.1 (`rust-toolchain.toml`). TS side: existing `@wagner/shared` (ESM, `tsc` strict). |
| Primary Dependencies | Present: `serde 1` (derive), `serde_json 1`, `ulid 1`, `chrono 0.4`, `jsonschema 0.18`, `async-trait`, `iroh` (for `NodeId`). **New (build-time):** `schemars` 1.x (Rust type → JSON Schema). **New (TS dev):** `json-schema-to-typescript` (schema → `.ts`). TS validation reuses `ajv 8.17` (already in `shared`). |
| Storage | N/A — contracts/types only. No persistence is built here (the Envelope is designed to be persistable/recordable later; FR-018, §7 seam #1). |
| Testing | `cargo test` (serde round-trip, schema validate, no-handle guard, additive-versioning) per D-TEST-1; `vitest` + `tsc` for the generated TS binding compile/validate per D-TEST-2/3. |
| Target Platform | Edge host (macOS/Linux dev), headless library — no Tauri/UI link (`lib.rs`). |
| Project Type | Single-crate module addition (`wagner-edge-host`) + a generated TS artifact in the `shared` workspace. |
| Performance Goals | N/A — contracts carry no runtime hot path. Serde derive + compile-time schema embed only. |
| Constraints | Plain serializable data only — no `JoinHandle`/`AppHandle`/channel-sender/closure in any `Event`/`Command` (FR-018, §7). Additive-versioning: a `stable` type's schema MUST NOT gain a required field or remove/retype one (FR-017). All payloads draft-2020-12 `additionalProperties:false` (FR-015, Article X). |
| Scale / Scope | v1 structural contract: 6 namespaces (Run/Goal/Vault/Voice/Ui + Ext), 3 stream kinds, 7 capabilities, 3 stability tiers. Leaf `Event`/`Command` variants are **deferred** to `011` P0 and land additively (FR-006/FR-007). |

> Unknowns are marked `NEEDS CLARIFICATION` and resolved by Phase 0 research below. Open: serde enum tagging style → schema/TS shape (locked to adjacently-tagged here, pending the Phase-0 three-way-compat confirm).

---

## Constitution Check

Gates are Wagner's actual articles (`docs/spec/constitution.md §Gates`), not the generic template list. This is a **contracts/types-only** phase: several runtime gates are satisfied trivially (no runtime path) and one (VIII) is reconciled explicitly below.

- [x] **Gate I — Test-First:** `tasks.md` (Phase 4) will list each serde/schema/guard test task ID before the type it covers. The plan commits to RED-first per type (Article I).
- [x] **Gate II — Evidence-Driven:** every FR/SC is cited to `runtime-architecture.md` (LOCKED) or a `D-*`/Article ID; this plan introduces no vague adjective. spec-challenger confirms at Phase 5.
- [ ] **Gate III — CRITICAL-Resolved:** open until spec-validator returns READY (Phase 6). This plan introduces no constitution violation; both non-blocking spec markers are resolved (FR-019 → compile-time, below; EC-001 size enforcement → scoped to the `011` P1 intake boundary, spec EC-001). Phase 5 challenge findings disposed in `challenges.md` (3 CRITICAL + 4 HIGH → 0 open).
- [x] **Gate IV — Independent-MVP:** US1 (P1) has a concrete Independent Test (serde round-trip + schema accept/reject + no-handle guard) — `spec.md` US1.
- [x] **Gate V — Simplicity:** the contract is a **module** in the existing crate, not a new crate or service; no broker/actor-framework/datastore added (§6 non-goals upheld). Two build-time tooling deps (`schemars`, `json-schema-to-typescript`) are added — see Complexity Tracking for the rejected-alternative analysis (they are dev/build surface, not operational surface).
- [x] **Gate VI — Edge-Autonomy:** contracts are pure types; no hub round-trip, no metered-API-key env var on any path (D-SEC-1). Trivially satisfied — nothing executes.
- [x] **Gate VII — Dependency-Direction:** types live under `edge/host` (inside `platform/`); the generated TS in `shared/contracts/` is **standalone pure types** that import nothing from `platform/`. The existing dependency-direction test's scope covers the generated file.
- [x] **Gate VIII — Event-Sourced (RECONCILIATION — see below):** the Gate VIII replay-equals-snapshot test **already exists and passes** in the carried reducer — `shared/reducer/run-reducer.test.ts:38` (SC-005 replay-from-empty byte-identical to the live fold) and `shared/reducer/remote-events.test.ts:69` ("replay equals incremental fold (Article VIII)"). This contracts-only phase adds **no new** reducer/log/replay test because the gate is already green; 013's contracts are designed to keep future `Event`s foldable by that same pure reducer. **Not deferred — satisfied today.**
- [x] **Gate IX — Privacy-Boundary:** `Envelope` carries `Scope{user, workspace}` (FR-002/FR-004) — the multi-tenant filter seam — and the contract holds **no** code/diff/transcript field. `Ext` payloads are bounded by `additionalProperties:false` against their registered schema, so an extension cannot smuggle fields past the boundary (EC-005). The metadata+learnings sync schema is a separate track (D-DATA-1).
- [x] **Gate X — Schema-Validated:** every `Event`/`Command`/manifest payload gets a declared draft-2020-12 `additionalProperties:false` schema (FR-015), validated at the boundary via `jsonschema` (Rust) / `ajv` (TS). This gate is the spine of the whole plan.

### Article VIII reconciliation — transient bus vs event-sourced truth

The apparent tension (`runtime-architecture.md §0.1/§6`: "the runtime is in-process and **transient** … we deliberately do NOT build an event-sourced/CQRS bus" vs Article VIII NON-NEGOTIABLE: "run state MUST be derived from an append-only event log; the log is the source of truth") resolves cleanly, and this contract is what makes the resolution hold:

- **The bus is a delivery mechanism, not the log.** It is transient and never persists (§0.1, §6). Article VIII is **not** a claim about the bus.
- **The source of truth is the durable plane** — loro CRDT + per-run JSON, folded by the **existing pure reducer** (`shared/reducer/run-reducer.ts`, the carried Article VIII spine; D-STORE-2). That plane is event-sourced; the bus merely carries the same `Event`s to it (and to the UI projection, and to any future opt-in recorder participant — §6).
- **013's job is to make that durable fold possible and replayable:** `Event`/`Command` are plain serializable data with no embedded handle (FR-018, §7 seam #1) → foldable by a pure reducer and recordable; `Envelope.stream + seq` give deterministic per-stream ordering (FR-001/EC-002); the per-payload schema-version (FR-016, D-STORE-1) makes a recorded log replayable across versions.
- **Therefore:** the bus stays transient AND Article VIII holds, because the log/truth lives in the durable CRDT+JSON plane, not the bus. The Gate VIII *test* (reducer pure + replay-equals-snapshot) **already exists and passes today** — `shared/reducer/run-reducer.test.ts:38` (SC-005 replay-from-empty byte-identical to the live fold) and `shared/reducer/remote-events.test.ts:69` ("replay equals incremental fold (Article VIII)"). This phase adds no *new* replay test because the gate is already green; 013's contracts are designed so future `Event`s remain foldable by that same pure reducer. No `## Complexity Tracking` entry is required — the gate passes; it is neither deferred nor violated. (Corrects an earlier draft that framed this as "out of scope by phase" — challenge H1.)

---

## Project Structure

New module inside the existing crate (no new crate — Gate V), plus the exported schema catalog and the generated TS bindings.

```
edge/host/
├── Cargo.toml                      # + schemars 1.x (build-time derive)
├── src/
│   ├── lib.rs                      # + `pub mod bus;`
│   └── bus/                        # NEW — the public contract (types only, no behaviour)
│       ├── mod.rs                  # re-exports; module docs; the bin/test that exports schemas
│       ├── envelope.rs             # Envelope, EventId(Ulid), Timestamp, StreamId, Scope
│       ├── event.rs                # Event (Run/Goal/Vault/Voice/Ui + Ext) — namespace scaffolding
│       ├── command.rs              # Command (same namespaces + Ext)
│       ├── participant.rs          # ParticipantId, ParticipantKind, Agent trait sig, Subscription
│       └── manifest.rs             # PluginManifest, Capability (closed v1 set), StabilityTier
├── schemas/                        # EXISTING — exported catalog; new *.schema.json land here
│   ├── (5 carried schemas, untouched)
│   └── bus/                        # generated draft-2020-12 schemas for the contract types
└── tests/
    └── unit/
        ├── bus_serde_roundtrip.rs  # SC-001 — round-trip per structural variant
        ├── bus_schema_validate.rs  # SC-002/003/006 — accept/reject, Ext-against-registered
        ├── bus_no_handle_guard.rs  # SC-004 — no non-serializable handle (§7 seam #1)
        └── bus_additive_version.rs # SC-005 — stable type adds optional field, old payload still valid

shared/
├── package.json                    # + "gen:contracts" script; devDep json-schema-to-typescript
└── contracts/                      # NEW — generated TS bindings (pure types; Gate VII safe)
    ├── index.ts                    # generated barrel
    └── *.d.ts                      # generated from edge/host/schemas/bus/*.json
```

**Structure Decision:** A module, not a crate — the contract has one consumer today (the host) and Gate V mandates the smallest layout; extract-to-crate is additive if `hub` ever needs the Rust types directly. Per-concern files match `runtime-architecture.md §2` ("each variant lives in its own module") without over-splitting (5 source files for 6 namespaces + identity + manifest). Schemas reuse the existing `edge/host/schemas/` catalog dir (FR-019); a `bus/` subdir keeps the new contract schemas separate from the 5 carried ones. Generated TS lands in `shared/contracts/` — the natural home for cross-surface types consumed by `edge/ui` and `hub`, and pure enough to satisfy Gate VII.

---

## Phase 0 — Research

Three genuine unknowns. Output to `research.md` only if the engineer wants it persisted (offered at the end).

```
R1 — Schema/codegen three-way compatibility on draft 2020-12 (CONFIRM, not open).
  schemars 1.x supports draft 2020-12 and the decision is locked (below); this is a confirmation,
  not an open design question. Confirm by smoke test: a (schemars version, jsonschema 0.18,
  ajv 8 / json-schema-to-typescript) tuple that all agree on draft 2020-12 AND on the
  adjacently-tagged enum representation. The confirmation is folded into task T001 and GATES the
  first US1 test (T005) — see tasks.md.
  Output: Decision (pinned versions + tagging) / Rationale / Alternatives.

R2 — iroh NodeId as a contract field.
  Resolve: iroh::NodeId serde form (string round-trip) and its JSON-Schema string shape; confirm
  it round-trips and validates without pulling iroh runtime into the pure-types boundary.
  Output: Decision (use iroh::NodeId vs a NodeId(String) newtype) / Rationale / Alternatives.

R3 — TS ergonomics of the generated bindings.
  Resolve: json-schema-to-typescript output for additionalProperties:false + adjacently-tagged
  oneOf → a usable TS discriminated union for Event/Command.
  Output: Decision (generator config) / Rationale / Alternatives.
```

Resolutions already locked by this plan (recorded so research only *confirms* them):

- **R1/R3 tagging → adjacently-tagged enums.** `#[serde(tag = "type", content = "data")]` on `Event`/`Command` (and the `Ext{ns,name,version,payload}` variant as a structured object). Rationale: externally-tagged serde (the default, shown illustratively in `runtime-architecture.md §2`) yields `oneOf` maps that generate awkward TS; adjacently-tagged gives a clean discriminated union (`{type, data}`) that `json-schema-to-typescript` renders idiomatically and TS narrows on. Alternatives: internally-tagged (fails for non-struct/newtype payloads), externally-tagged (poor TS ergonomics).
- **FR-019 → compile-time registry for v1.** The exported schema catalog is produced at build/test time from the Rust types (`schemars`); `Ext{ns,name,version,payload}` names a schema resolved from that compile-time catalog. Load-time (config-discovered) registry is **deferred** — additive, lands when third-party plugins do. Rationale: `runtime-architecture.md §10` ("start compile-time; revisit when external plugins actually land") + Gate V (the solo author authors all plugins near-term; a load-time plugin loader is operational surface YAGNI for contracts-only). This resolves the spec's FR-019 `[NEEDS CLARIFICATION]`.

---

## Phase 1 — Design & Contracts

### 1.1 Data Model (prose; `data-model.md` only on request)

- **Envelope** (FR-001/002): `{ id: EventId(Ulid), ts: Timestamp, origin: ParticipantId, stream: StreamId, seq: u64, scope: Scope, payload: Event }`. `additionalProperties:false`. Carries `origin`+`scope` from v1 (§7 seam #2). Validation: all fields required; `seq` monotonic per `stream` (the contract *carries* `seq`; enforcement is `011` P1).
- **Event** (FR-006/008/009): adjacently-tagged enum over `Run | Goal | Vault | Voice | Ui | Ext`. Namespace set is the **stable structure**; leaf variants per namespace are deferred to `011` P0 (derived from the 7 `wagner://*` channels + voice), added additively. Facts are past-tense names. `Ext{ ns, name, version, payload }` carries an extension fact whose `payload` validates against a registered schema.
- **Command** (FR-007/008/009): same namespaces + `Ext`; imperative names; leaves deferred to `011` P0 (from the migrated Tauri handlers + voice intake).
- **ParticipantId** (FR-003): `{ node: iroh::NodeId, kind: ParticipantKind, name: String, instance: Ulid }`. `ParticipantKind ∈ {GoalLoop, Agent, Connector, Scheduler, Ui, System}`. (`name` is `String`, not the doc's illustrative `CompactString` — no `compact_str` dep at this scale; revisit only if profiling shows it matters. `ponytail:` String, newtype-or-CompactString only if measured.)
- **Scope** (FR-004): `{ user, workspace }` — exactly two fields in v1; org tier / further fields are additive when multi-tenant (§13.4) lands.
- **StreamId** (FR-005): closed enum `{ Run(..), Agent(..), Workspace(..) }`, `Run` the common case; new kinds additive.
- **Subscription** (FR-011): a topic/namespace + filter selector (e.g. `vault.*`, `ext.slack.*`, `stream:<id>`) — the *type shape* only; matching is `011` P1.
- **PluginManifest** (FR-012): `{ participants_provided: [..], emits: [Namespace], subscribes: [Namespace], registered_schemas: [SchemaRef], capabilities: [Capability], stability: StabilityTier }`. Empty `capabilities` is **valid** (pure subscriber — EC-004).
- **Capability** (FR-014): closed enum, exactly 7: `Network, ProcessSpawn, VaultRead, VaultWrite, FsRead, FsWrite, SecretsRead`. Coarse — no per-path/per-host scope in v1 (additive later). Declared, not enforced (sandbox deferred).
- **StabilityTier** (FR-010): `{ stable, experimental, internal }`; new type defaults to `experimental`; promotion to `stable` is an explicit change that binds the no-break rule (FR-017).
- **Agent (trait)** (FR-013): signature only — `name(&self)`, `subscriptions(&self)`, `init/handle/shutdown` async lifecycle. Implementing/registering participants is `011` P4, out of scope.

Every struct/enum derives `Serialize, Deserialize, schemars::JsonSchema` and carries `#[serde(deny_unknown_fields)]` so serde *and* the derived schema both enforce closed objects (FR-015).

### 1.2 Interface Contracts

- **The exported JSON Schema catalog** (`edge/host/schemas/bus/*.json`, FR-019): the discoverable "what can I emit/subscribe to" catalog AND the single source for TS bindings. Produced by a `schemars` export step (a small `bus::export_schemas` invoked by a test/bin that writes the files; the committed files are diffed against a fresh export to catch drift — SC-002). Versioning: additive only; a `stable` type's schema never gains a required field (FR-017, enforced by the additive-version regression test SC-005). Error model: a payload that violates its schema is rejected at the boundary before write/emit/sync (FR-015, EC-003/005).
- **The Rust→TS binding contract** (US3, SC-007): `json-schema-to-typescript` reads `edge/host/schemas/bus/*.json` → emits `shared/contracts/*.d.ts`; a `vitest`/`tsc` check compiles every `stable`-tier binding and validates a representative payload with `ajv`. Drift (Rust type changed, schema/TS stale) fails CI at regen-diff.
- **The `Ext` extension contract** (US2, FR-009, SC-006): an `Ext{ns,name,version,payload}` validates against the schema its plugin registers, with **zero** edits to the core `Event` enum source — the extension seam.

### 1.3 Cross-Cutting Concerns

#### Observability
- **Log fields / metric names / trace spans:** N/A for this phase — contracts carry no runtime behaviour, so there is nothing to log, meter, or span yet. The *fields that make runs observable later* are baked into the contract: `Envelope.{id, origin, stream, seq, scope, ts}` are the structured identifiers every downstream log/metric will key on (D-OBS-1). No metric is emitted here.

#### Security
- **Trust boundary:** the schema-validation point (FR-015) — any `Event`/`Command`/manifest crossing into disk, a UI surface, or sync is validated draft-2020-12 `additionalProperties:false`. `Ext` payloads are validated against their registered schema (EC-005, malicious/oversized-field defense).
- **Authentication / Authorisation:** none enforced in contracts. `ParticipantId.node` (iroh node key) is the identity the future authorize step (intake, `011` P3) will key on. Capabilities are **declared, not enforced** — the sandbox is deferred (`specs/012` §13.7); this is an accepted, recorded gap (Trade-offs, below), made additive by carrying the declaration now.
- **Secrets:** none. The `SecretsRead` capability is a declaration token, not a secret (D-SEC-2 — no credential is inlined).

#### Failure Modes
- **Validation rejection (EC-003):** an invalid payload is rejected at the boundary; no partial/invalid payload is visible. Atomic write (D-RES-1) is downstream (`011`).
- **Version skew (SC-005):** a `stable` type gaining a required field is the one breaking change the contract forbids; caught by the additive-version regression test (regenerate schema, assert old payloads still validate).
- **Stale ordering (EC-002):** the contract carries `stream + seq` so the reducer's stale-event guard (D-RES-3) is *expressible*; enforcement is `011` P1.
- **Graceful degradation / retries / backoff:** N/A — no dependency is called by a type.

---

## Complexity Tracking

Two build-time tooling dependencies are added. Neither is a service, engine, datastore, or operational surface (Gate V is about those); recorded here for the rejected-alternative analysis.

| Addition | Why Needed | Simpler Alternative Rejected Because |
|----------|------------|--------------------------------------|
| `schemars` 1.x (Rust, build/derive) | FR-015/FR-017/SC-002/SC-005 require a draft-2020-12 schema per contract type that stays in lockstep with the Rust type as it additively versions over years. Deriving the schema from the type makes the Rust type the single source; a regen-diff test catches drift. | Hand-authoring JSON schemas (today's `include_str!` pattern) keeps Rust types, JSON schemas, and TS types in **triplicate** — the exact silent-drift failure Article X exists to prevent. Acceptable for 5 frozen carried schemas; unacceptable for the additively-versioned platform SDK surface. |
| `json-schema-to-typescript` (TS devDep) | FR-019 mandates the exported schemas be **the source** for generated TS bindings; SC-007 requires every `stable` type to have a compiling TS binding. | Hand-writing TS types, or `ts-rs`/`typeshare` (Rust→TS direct), both violate FR-019 ("schemas MUST be the source") and reintroduce a second Rust-derived representation that can disagree with the schema. Schema-as-source is the contract. |

Not added (recorded so the choice is visible): `compact_str` (use `String` — no measured need), `inventory` (compile-time catalog is the schemars export, not a runtime plugin-registration crate — load-time registry deferred per FR-019 decision).

---

## Optional Artifacts

Created on engineer request, not by default:

- [ ] `data-model.md` — the §1.1 entities as a field-level schema table.
- [ ] `contracts/` — the exported `*.schema.json` are the real contract artifact (land under `edge/host/schemas/bus/` during build, not pre-authored here).
- [ ] `research.md` — the R1–R3 findings, if you want them persisted before `/spec tasks`.
- [ ] `quickstart.md` — a walkthrough: construct an `Envelope` per namespace, round-trip, validate, generate + compile the TS binding.
