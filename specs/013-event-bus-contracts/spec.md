# Feature Specification: Event Bus Contracts (Phase 0)

**Feature Branch:** `013-event-bus-contracts`
**Created:** 2026-06-18
**Status:** Draft
**Input:** Engineer's original description: "Phase 0 of the platform-foundation buildout (`specs/012-platform-foundation/design.md`): the public CONTRACTS of the event bus — the typed Envelope, the core Event/Command taxonomy (namespaced enums Run/Goal/Vault/Voice/Ui + the `Ext{ns,name,version,payload}` extension variant), the uniform Plugin manifest (Agent trait + declared capabilities + registered JSON schemas + stability tiers), the capability vocabulary v1, and JSON-Schema export at the Rust→TS boundary. Bound by `docs/runtime-architecture.md` (LOCKED) §2/§3/§7; amends `specs/011-runtime-foundation` P0/P4. Scope is contracts/types only — NO behavior wiring."

> **Spec authoring rules.** WHAT/WHY only; mechanism (serde, codegen tool, module layout, the bus loop) lives in plan.md. Every requirement is cited to `docs/runtime-architecture.md` (LOCKED) or a constitution/defaults ID per Article II. **No invented priorities, Independent Tests, or enum sets** — each is flagged `[NEEDS CLARIFICATION]`. No marker budget; worked through in `/spec clarify`.

---

## User Scenarios & Testing *(mandatory)*

> **Scope guard:** these stories are about the **contract existing, being type-safe, serializable, and schema-validated** — not about the bus routing, intake dispatching, or the registry loading anything (that behavior is `011` P1+, out of scope here).

### User Story 1 — Author a first-party participant against the typed contract (Priority: P1)

A first-party engine developer adds a new participant (an agent, connector, scheduler, or the UI gateway). To do so they: construct an `Envelope` carrying a typed, namespaced `Event` (fact) or react to a typed `Command` (intent); and declare a **Plugin manifest** stating the participants they provide, the namespaces they emit/subscribe, the schemas they register, the capabilities they request, and a stability tier. The contract makes "create an agent" and "add an integration" the identical authoring move (`runtime-architecture.md` §3).

**Why this priority:** Without a typed contract a developer can author a participant against (`Envelope` + core `Event`/`Command` + manifest + `Agent` trait + schemas), nothing else in the runtime has types to build on. This is the minimum shippable contract.

**Independent Test:** Construct an `Envelope` carrying a typed core `Event` of each namespace (Run/Goal/Vault/Voice/Ui — one representative seed variant each) plus a Plugin manifest; the test suite proves serde round-trip + JSON-Schema accept/reject + the no-handle seam guard all pass — verifying the contract **structure** (the namespaces + `Ext` seam + `Agent` trait + manifest) is authorable and boundary-safe, with concrete leaf variants landing additively in `011` P0.

**Acceptance Scenarios:**

1. **Given** a core `Event` value of each namespace (Run/Goal/Vault/Voice/Ui), **When** it is serialized and deserialized, **Then** the result is byte-equal to the input (serde round-trip) — `runtime-architecture.md` §2, `011` Step 0, D-TEST-1.
2. **Given** a payload that conforms to its declared JSON Schema, **When** it is validated, **Then** validation passes; **Given** a payload with an extra or wrong-typed field, **When** it is validated, **Then** validation fails (`additionalProperties:false`) — Article X, D-OBS-1, D-SEC-3.
3. **Given** any `Event` or `Command` variant, **When** a guard inspects it, **Then** it contains no non-serializable handle (no `JoinHandle`/`AppHandle`/channel-sender/closure) — `runtime-architecture.md` §7 seam #1.
4. **Given** a Plugin manifest value, **When** it is constructed, **Then** it expresses participants-provided, emitted/subscribed namespaces, registered schema references, requested capabilities, and a stability tier — `specs/012` design §Extension model.

---

### User Story 2 — Extend the taxonomy without changing the core (Priority: P3)

A developer (first-party today; third-party someday) introduces a new event/command type in their own namespace via `Event::Ext { ns, name, version, payload }` / `Command::Ext { … }`, where `payload` validates against a JSON Schema their plugin registers — without editing or recompiling the core enums (`runtime-architecture.md` §2).

**Why this priority:** The `Ext` path matters most for third-party plugins, which are deferred ("someday", `specs/012`); near-term the operator can add core variants directly, so this is a thin increment that ships last.

**Independent Test:** Emit an `Ext{ns,name,version,payload}` event whose payload matches its registered JSON Schema, with zero changes to the core `Event` enum source — verifying the extension seam works without a core PR.

**Acceptance Scenarios:**

1. **Given** an extension event `Ext{ns,name,version,payload}` whose payload matches a registered schema, **When** it is validated, **Then** validation passes and the core `Event` enum is unchanged — `runtime-architecture.md` §2.
2. **Given** an extension payload that violates its registered schema, **When** it is validated, **Then** validation fails at the boundary — Article X.
3. **Given** a stable core type, **When** an optional field is added and its `version` bumped, **Then** the prior schema still validates existing payloads (additive-versioning; no break to a `stable` type) — `runtime-architecture.md` §2.

---

### User Story 3 — Consume the contract from TypeScript (Priority: P2)

A UI/TypeScript developer consumes the event/command contract through typed bindings generated from the exported JSON Schemas, so Rust→TS shape drift is caught at the boundary rather than in the frontend (`architecture.md` §3, Article X). The exported schema set is the discoverable catalog of "what can I emit / subscribe to" (`runtime-architecture.md` §2).

**Why this priority:** The Rust→TS boundary is how the existing UI keeps working on the bus (`architecture.md` §3); it is load-bearing but depends on US1's types existing first, so P2.

**Independent Test:** Generate TS bindings from the exported schemas; every `stable`-tier core type compiles against its binding and a representative payload validates — verifying the Rust→TS boundary catches shape drift.

**Acceptance Scenarios:**

1. **Given** the exported schema set under `edge/host/schemas/`, **When** TS bindings are generated and compiled, **Then** every `stable`-tier core type has a corresponding typed binding — Article X, `runtime-architecture.md` §2.
2. **Given** a committed schema file, **When** the schema-validation test suite runs, **Then** the schema and a representative payload validate against it — D-TEST-3.

---

### Edge Cases

- **EC-001 (boundary):** A core `Event`/`Command` enum carries a variant added after v1. Expected: older consumers ignore unknown optional fields; required fields are never added to a `stable` type (additive-versioning, `runtime-architecture.md` §2). Payload **size enforcement is out of scope for this contracts/types-only phase** — no boundary accepts external bytes yet, so there is nothing to bound here; the max-payload limit is set at the intake boundary in `011` P1, where untrusted bytes first arrive and the validator could otherwise be made to materialize an unbounded blob. (Article II: the constraint is scoped + deferred to its enforcement point, not left dangling.)
- **EC-002 (concurrency):** Two participants emit events on the same `stream` concurrently. Expected: the contract carries `stream` + monotonic `seq` so per-stream ordering is expressible; the reducer's stale-event guard (older event never clobbers newer for the same actor) applies — `runtime-architecture.md` §2, D-RES-3. [This spec defines the *fields*; enforcement is `011` P1.]
- **EC-003 (failure):** A payload fails schema validation at the boundary. Expected: it is rejected before write/emit/sync; no partial/invalid payload is visible — Article X, D-RES-1.
- **EC-004 (zero-state):** A plugin manifest declares zero capabilities. Expected: **valid** — a participant that requests nothing (e.g. the UI gateway, the voice-projection participant) is legitimate; an empty capability set means "requests no capabilities." Source: clarified 2026-06-18.
- **EC-005 (malicious input):** An extension registers a schema, then emits a payload with an unexpected/oversized field. Expected: rejected by `additionalProperties:false` validation against the registered schema — Article X. Malicious *schema registration* (a recursive/circular `$ref` that could drive the validator to OOM or infinite recursion) is **not a v1 attack surface**: schemas are compile-time artifacts (FR-019 compile-time-registry decision), not load-time registered from untrusted plugins — revisit cycle-detection / bounded evaluation depth when a load-time registry lands. [Capability over-request handling is deferred — sandbox is someday, `specs/012` design.]

---

## Functional Requirements *(mandatory)*

**Envelope & identity**

- **FR-001:** The contract MUST define a concrete `Envelope` carrying: a unique sortable id, a timestamp, `origin` (a `ParticipantId`), `stream` (an ordering scope), a per-stream monotonic `seq`, `scope` (owner + workspace), and a typed `payload` (`Event`). Source: `runtime-architecture.md` §2.
- **FR-002:** The `Envelope` MUST carry `origin` (`ParticipantId`) and `scope` (user + workspace) from v1, so multi-tenant later is a subscription filter, not a record migration. Source: `runtime-architecture.md` §7 seam #2; D-DATA-1.
- **FR-003:** `ParticipantId` MUST identify which peer/machine (an iroh node identity), the participant kind (one of GoalLoop, Agent, Connector, Scheduler, Ui, System), a stable logical name, and a per-instance id. Source: `runtime-architecture.md` §3.
- **FR-004:** `scope` MUST be `Scope { user, workspace }` — the multi-tenant seam. v1 carries exactly these two fields; an org tier or further fields are added additively when multi-tenant (§13.4) lands. Source: `runtime-architecture.md` §2/§5; clarified 2026-06-18.
- **FR-005:** `stream` MUST be a typed `StreamId` over exactly three closed kinds — `Run`, `Agent`, `Workspace` — with `Run` the common case. New kinds are added additively. Source: `runtime-architecture.md` §2; clarified 2026-06-18.

**Event / Command taxonomy**

- **FR-006:** The contract MUST define a core `Event` enum namespaced into exactly these v1 namespaces: `Run`, `Goal`, `Vault`, `Voice`, `Ui`, plus `Ext` (FR-009). The namespace set is the **stable contract structure**. The concrete leaf variants within each namespace are enumerated during `011` P0 implementation — derived from the 7 `wagner://*` channels + the new voice events — and added additively (FR-017). Source: `runtime-architecture.md` §2/§10, `011` Step 0; clarified 2026-06-18 (namespaces locked, leaves deferred to implementation).
  - *Reconciliation (the LOCKED-doc §2 examples are illustrative, not the set):* `runtime-architecture.md` §2's Rust code example (`Goal/Agent/Integration/Scheduler/Vault/Ui`) and its prose list (`Goal/Run/Vault/Scheduler/Ui, …`) are **illustrative** — §10 explicitly leaves "the concrete first cut of the core `Event`/`Command` enums" open for the build phase, which this clarification fills; the v1 set above supersedes both. `Agent` and `Integration` are **participant kinds** (`ParticipantKind`, FR-003), not Event namespaces; `Scheduler` is likewise a `ParticipantKind`, so scheduler-emitted facts land under `Run` (or a future additive `Scheduler` namespace) — it is intentionally absent from the v1 Event set. `Voice` is added (the 7-channel + voice derivation, `011` Step 0).
- **FR-007:** The contract MUST define a core `Command` enum namespaced over the same v1 namespaces as `Event` (+ `Ext`). Leaf command variants are enumerated during `011` P0 — derived from the migrated Tauri action handlers + voice intake — and added additively. Source: `runtime-architecture.md` §2/§10; clarified 2026-06-18 (namespaces locked, leaves deferred).
- **FR-008:** Facts (`Event`) MUST be named past-tense (e.g. `RunFinished`, `PrOpened`); commands (`Command`) MUST be named imperative (e.g. `StartRun`, `PostMessage`). Source: `runtime-architecture.md` §2.
- **FR-009:** The taxonomy MUST support extension types via `Event::Ext { ns, name, version, payload }` and `Command::Ext { … }`, where `payload` validates against a schema registered by the contributing plugin. Source: `runtime-architecture.md` §2.
- **FR-010:** Every core and extension `Event`/`Command` type MUST carry a stability-tier annotation, one of `stable | experimental | internal`. A newly added type defaults to `experimental`; promotion to `stable` (which binds the no-break rule FR-017) MUST be an explicit, deliberate change; `internal` is for engine-private types. Source: `runtime-architecture.md` §2; clarified 2026-06-18.
- **FR-011:** Subscriptions MUST be expressible by topic/namespace + filter (e.g. `vault.*`, `ext.slack.*`, `stream:<id>`), not by matching a single god-enum. Source: `runtime-architecture.md` §2. **Motivated by FR-013:** the `Agent` trait's `subscriptions()` returns `Vec<Subscription>`, so US1's `NoopAgent` (AS-3) constructs and exercises it — `Subscription` is load-bearing for the trait signature, not a standalone speculative type. [This spec defines the `Subscription` type *shape*; matching behavior is `011` P1, so the v1 filter selector stays minimal.]

**Plugin manifest & capabilities**

- **FR-012:** The contract MUST define a uniform **Plugin manifest** declaring: participants provided (each an `Agent`), event/command namespaces emitted and subscribed, references to the JSON Schemas the plugin registers, the capabilities the plugin requests, and a stability tier. Source: `specs/012` design §Extension model. (This is the addition that amends `011` P0/P4.)
- **FR-013:** The contract MUST define the `Agent` trait as the single participant contract (logical name, declared subscriptions, and init/handle/shutdown lifecycle signatures). Source: `runtime-architecture.md` §3. [This spec defines the trait *signature*; implementing/registering participants is behavior — `011` P4, out of scope.]
- **FR-014:** The contract MUST define a **capability vocabulary v1** — a **closed, enumerated** set of coarse capability kinds a plugin may request in its manifest, honored on trust today and designed for sandbox enforcement later. The v1 set is exactly: `network`, `process.spawn`, `vault.read`, `vault.write`, `fs.read`, `fs.write`, `secrets.read`. Capabilities are **coarse** — no per-path or per-host scoping in v1 (scoping is added additively when the sandbox lands). The set grows additively (new kinds added; none removed or retyped for a `stable` manifest). Source: `specs/012` design §Extension model ("declare capabilities now, enforce later"); clarified 2026-06-18.

**Serialization, schema, versioning**

- **FR-015:** Every `Event`, `Command`, and manifest payload MUST validate against a declared JSON Schema (draft 2020-12, `additionalProperties:false`) before it is written to disk, emitted to a UI surface, or synced. Source: Article X, D-OBS-1, D-SEC-3.
- **FR-016:** Every persisted contract payload MUST carry a schema-version identifier (e.g. `"schema": "<name>.v1"`). Source: D-STORE-1.
- **FR-017:** Schemas MUST evolve additively only: add optional fields and bump `version`; a `stable` type's schema MUST NOT gain a required field or remove/retype a field. Source: `runtime-architecture.md` §2/§7.
- **FR-018:** Every `Event` and `Command` value MUST be plain serializable data — it MUST NOT embed a `JoinHandle`, `AppHandle`, channel sender, or closure. Source: `runtime-architecture.md` §7 seam #1.
- **FR-019:** The exported schema set (`edge/host/schemas/`) MUST serve as the discoverable catalog of emittable/subscribable types, and MUST be the source for generated TypeScript bindings at the Rust→TS boundary. Source: `runtime-architecture.md` §2, `architecture.md` §3, Article X. The extension schema registry is **compile-time** for v1 — the exported catalog is produced at build/test time from the Rust types, and `Ext{ns,name,version}` resolves its schema from that compile-time catalog; a load-time (config-discovered) registry is deferred and lands additively when third-party plugins do. Source: `runtime-architecture.md` §10 ("start compile-time; revisit when external plugins actually land"); clarified 2026-06-18 (plan.md R1 / FR-019 resolution).

---

## Success Criteria *(mandatory)*

- **SC-001:** 100% of core `Event` and `Command` variants pass a serde round-trip test (deserialize(serialize(x)) == x). Source: D-TEST-1.
- **SC-002:** 100% of core `Event`/`Command`/manifest payload types have an exported JSON Schema, and each schema + a representative payload validate in the test suite. Source: Article X, D-TEST-3.
- **SC-003:** 100% of schema-invalid payloads (extra field, wrong type) are rejected at the boundary (`additionalProperties:false`). Source: Article X.
- **SC-004:** A guard test proves 0 `Event`/`Command` variants contain a non-serializable handle. Source: `runtime-architecture.md` §7.
- **SC-005:** An additive-versioning regression test passes: adding an optional field to a `stable` type does not break validation of payloads written against the prior schema. Source: `runtime-architecture.md` §2.
- **SC-006:** A new `Ext{ns,…}` event validates against its registered schema with 0 changes to the core enum source. Source: `runtime-architecture.md` §2.
- **SC-007:** 100% of `stable`-tier core types have a generated TypeScript binding that compiles. Source: Article X.
- **SC-008:** 100% of `Event`/`Command` types carry a stability tier and (for persisted payloads) a schema-version identifier. Source: `runtime-architecture.md` §2, D-STORE-1.

---

## Key Entities

- **Envelope:** the unit carried by the bus; wraps a typed payload with identity, ordering, and scope (FR-001).
- **Event:** a namespaced, past-tense fact (Run/Goal/Vault/Voice/Ui + `Ext`).
- **Command:** a namespaced, imperative intent (+ `Ext`).
- **ParticipantId:** stable identity of a participant — node + kind + name + instance (FR-003).
- **Scope:** owner + workspace; the multi-tenant filter seam (FR-004).
- **StreamId / EventId / seq:** ordering scope, unique id, per-stream monotonic counter.
- **Subscription:** a topic/namespace + filter selector (FR-011).
- **Plugin manifest:** participants + namespaces + registered schemas + requested capabilities + stability tier (FR-012).
- **Agent (trait):** the uniform participant contract (FR-013).
- **Capability:** a declared, enumerable permission a plugin requests (FR-014).
- **StabilityTier:** `stable | experimental | internal` (FR-010).

---

## Assumptions

- The design is **bound by** `docs/runtime-architecture.md` (LOCKED) §2/§3/§7; this spec deepens it and MUST NOT contradict it. (Engineer-stated.)
- This spec **amends** `specs/011-runtime-foundation` P0/P4 to add the Plugin manifest + capability declaration; it does not replace `011`. (Engineer-stated.)
- Scope is **contracts/types only** — no bus routing, no intake dispatch, no registry loading, no participant behavior (those are `011` P1+). (Engineer-stated.)
- Third-party plugin **sandbox/registry is deferred** ("someday"); capabilities are **declared now, enforced later** — the v1 contract carries the declaration without enforcing it. (Engineer-stated, `specs/012` design.)
- The core taxonomy does **not** depend on the (in-progress) UI mocks; UI-projection event additions are additive and land in Phase 5. (Engineer-stated, `specs/012` design.)

### Defaults Applied

- `D-OBS-1` — every event validates against a `wagner-event`-style schema (Article X) → FR-015, SC-002.
- `D-STORE-1` — JSON state with a declared `schema`-version field → FR-016.
- `D-SEC-3` — external inputs validated against a declared schema before use → FR-015, EC-003/EC-005.
- `D-DATA-1` — default sync is metadata + curated learnings only → motivates `scope` on the envelope (FR-002).
- `D-TEST-1` — Rust host tested with `cargo test`, no live CLI → SC-001, acceptance tests.
- `D-TEST-3` — schema-validation test per committed schema + representative payload → SC-002.
- `D-RES-1` — atomic write/validate/rename for any persisted payload → EC-003.
- `D-RES-3` — stale-event guard (older event never clobbers newer for same actor) → EC-002.
- `D-PROJ-4` — Rust host + TypeScript frontend (Tauri) is the stack → US3, FR-019.
- `D-PROJ-3` — platform vocabulary (agent/planner, no floor-era terms) in all new names → naming throughout.

### Defaults Overridden

- *(none)*

---

## Out of Scope

- The in-process bus itself — `publish`/`subscribe`, channels, backpressure (`011` P1).
- Command intake/`dispatch` behavior (`011` P3).
- `Agent` registry loading, supervision, lifecycle execution (`011` P4).
- **Sandbox enforcement** of declared capabilities — deferred (someday; `specs/012` design §13.7).
- Concrete agent/connector/harness implementations.
- UI projection and the React port (`011` P7 / Phase 5).
- Data-layer wiring (loro / iroh / SurrealDB) — separate track.

---

## Dependencies

- **`docs/runtime-architecture.md` (LOCKED):** the binding design; if a contract choice here conflicts with it, this spec is wrong, not the doc.
- **`specs/011-runtime-foundation`:** this spec amends its P0/P4; `011`'s later phases consume these contracts.
- **JSON Schema draft 2020-12 tooling:** schema authoring + validation at the boundary (Article X). Failure mode: if a schema is missing/invalid, CI fails the schema-validation gate (D-TEST-3) and the payload is rejected (FR-015).
- **TypeScript binding generation (Rust→TS):** mechanism deferred to plan.md. Failure mode: missing binding → TS consumer compile failure (SC-007), caught at the boundary, not at runtime.

---

## Clarifications

Filled in by `/spec clarify`. Open markers gathered for the first session:

1. Priorities (P1/P2/P3) for US1–US3, and the Independent Test for each (action + value).
2. Concrete first-cut of the core `Event` enum variants (the 7 `wagner://*` channels + voice → namespaces) — `runtime-architecture.md` §10.
3. Concrete first-cut of the core `Command` enum variants — §10.
4. The **capability vocabulary v1** — the enumerated set (FR-014).
5. `stream` ordering-scope set (run/agent/workspace) and default (FR-005); `scope` fields beyond `{user, workspace}` (FR-004).
6. Default stability tier for a newly added type (FR-010); validity + default tier of an empty-capability manifest (EC-004).
7. Extension schema registry: compile-time vs load-time for v1 (FR-019) — partly a plan.md decision.

### Session 2026-06-18

- Q: User-story priorities + Independent Tests → A: **US1=P1** (author the contract — MVP), **US3=P2** (TS boundary), **US2=P3** (Ext extension path). Independent Tests defined per story: US1 = serde round-trip + schema accept/reject + no-handle seam guard; US3 = TS bindings compile against every `stable` schema + payload validates; US2 = `Ext{…}` validates against its registered schema with zero core-enum edits.
- Q: Capability vocabulary v1 → A: closed coarse set of 7 — `network`, `process.spawn`, `vault.read`, `vault.write`, `fs.read`, `fs.write`, `secrets.read`. No per-path/per-host scoping in v1 (added additively); set grows additively.
- Q: Enumerate Event/Command leaves now, or lock namespaces + defer? → A: **lock the 6 namespaces** (Run/Goal/Vault/Voice/Ui + Ext) as the stable structure; **defer leaf variants** to `011` P0 (derived from the 7 channels + Tauri handlers + voice), added additively.
- Q: `stream` kinds + `scope` fields → A: `StreamId ∈ {Run, Agent, Workspace}` (Run = common case); `Scope {user, workspace}` for v1; both extend additively.
- Q: default stability tier + empty-capability manifest → A: new type defaults to `experimental` (explicit promotion to `stable`); empty-capability manifest is **valid** (pure subscribers request nothing).
- Q: Extension schema registry — compile-time or load-time for v1 (FR-019)? → A: **compile-time** for v1 (the catalog is produced from the Rust types at build/test time; `Ext{ns,name,version}` resolves against it); a load-time (config-discovered) registry is deferred until third-party plugins land, added additively. Per `runtime-architecture.md` §10 + Gate V; resolution recorded in plan.md R1.
