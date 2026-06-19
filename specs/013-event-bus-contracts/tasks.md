# Tasks: Event Bus Contracts (Phase 0)

**Feature Branch:** `013-event-bus-contracts`
**Inputs:** `spec.md` (US1=P1, US3=P2, US2=P3), `plan.md` (Rust 2021 / toolchain 1.91.1; `schemars`→schema→`json-schema-to-typescript`; module `edge/host/src/bus/`), `docs/spec/constitution.md`
**Optional inputs:** none authored (data-model.md / research.md / quickstart.md are request-only).

> **Constitution Article I (NON-NEGOTIABLE):** every behaviour-changing task is preceded by a test task. For a types-only contract the "behaviour" is serde round-trip + schema accept/reject + no-handle + additive-versioning; those tests are written first and fail to compile (RED) until the types exist (GREEN).

---

## Phase 1 — Setup (Shared Infrastructure)

No story label.

- [ ] T001 [P] Add `schemars` (1.x, draft-2020-12 capable) to root `Cargo.toml [workspace.dependencies]` and `edge/host/Cargo.toml [dependencies]`, AND smoke-confirm the three-way compat tuple (plan R1): derive a schema for a throwaway adjacently-tagged enum and assert `jsonschema 0.18` (Rust) and `ajv 8` / `json-schema-to-typescript` (TS) all accept it as draft 2020-12. Pin the confirmed versions. **This confirmation gates T005** (the first US1 test compiles against the tuple).
- [ ] T002 [P] Add `json-schema-to-typescript` devDependency and a `"gen:contracts"` script (runs the generator over `edge/host/schemas/bus/*.json` → `shared/contracts/`) to `shared/package.json`.
- [ ] T003 [P] Register the four new test targets (`bus_serde_roundtrip`, `bus_schema_validate`, `bus_no_handle_guard`, `bus_additive_version`) as `[[test]]` entries in `edge/host/Cargo.toml` (Cargo does not auto-discover `tests/unit/`), and create `edge/host/schemas/bus/.gitkeep` + `shared/contracts/.gitkeep`.

---

## Phase 2 — Foundational (Blocking Prerequisites)

The compiling skeleton every story builds on. No types yet, so no test precedes it. No story label.

- [ ] T004 Add `pub mod bus;` to `edge/host/src/lib.rs`; create `edge/host/src/bus/mod.rs` with the contract's module rustdoc (namespaced, stability-tiered, additive-versioned, `#[serde(deny_unknown_fields)]`, "the exported schemas under `edge/host/schemas/bus/` ARE the catalog") and empty submodule declarations (`mod envelope; mod event; mod command; mod participant; mod manifest;` over empty files) so `cargo build` is green.

**Checkpoint:** crate compiles with an empty `bus` module; the four test targets resolve. User-story phases can begin.

---

## Phase 3 — User Story 1 — Author a first-party participant against the typed contract (Priority: P1) 🎯 MVP

**Goal:** the authorable, boundary-safe contract — `Envelope` + core `Event`/`Command` (each namespace constructible) + `PluginManifest` + `Agent` trait + exported schemas — so everything downstream has types to build on.
**Independent Test:** construct an `Envelope` carrying a typed core `Event` of each namespace (Run/Goal/Vault/Voice/Ui) plus a `PluginManifest`; the suite proves serde round-trip + JSON-Schema accept/reject + the no-handle seam guard all pass.

### Tests for User Story 1 (write FIRST, ensure they FAIL)

- [ ] T005 [P] [US1] Serde round-trip test in `edge/host/tests/unit/bus_serde_roundtrip.rs` — construct an `Envelope` carrying a representative `Event` of each namespace (Run/Goal/Vault/Voice/Ui) and a `Command` of each; assert `deserialize(serialize(x)) == x` byte-for-byte. Covers **SC-001, AS-1**.
- [ ] T006 [P] [US1] Schema validation test in `edge/host/tests/unit/bus_schema_validate.rs` — every core payload validates against its exported `edge/host/schemas/bus/*.json` via `crate::schema::validate`; a payload with an extra or wrong-typed field is **rejected** (`additionalProperties:false`); assert each persisted payload carries its `schema` version id and every contract type carries a stability tier. Covers **SC-002, SC-003, SC-008, AS-2, EC-003**.
- [ ] T007 [P] [US1] No-handle seam guard in `edge/host/tests/unit/bus_no_handle_guard.rs` — a `fn assert_plain<T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static>() {}` invoked for `Event`, `Command`, `Envelope` (a `JoinHandle`/`AppHandle`/sender/closure field would fail these bounds → fails to compile); plus a `NoopAgent` dummy `impl Agent` proving the trait signature is implementable, whose `subscriptions()` returns a `Vec<Subscription>` carrying a topic/namespace filter (e.g. `vault.*`) — assert that `Subscription` value serde-round-trips, exercising the `Subscription` shape (FR-011). Covers **SC-004, AS-3, FR-011, FR-013, FR-018**.
- [ ] T008 [P] [US1] Additive-versioning regression in `edge/host/tests/unit/bus_additive_version.rs` — add an `Option<T>` field to a `stable` contract type + bump its `version`; assert a payload written against the prior schema still validates (no required field added, none removed/retyped). Covers **SC-005, US2-AS-3, FR-017**.
- [ ] T009 [P] [US1] Manifest contract test in `edge/host/tests/unit/bus_schema_validate.rs` — a `PluginManifest` expresses participants-provided, emit/subscribe namespaces, registered schema refs, requested capabilities, and a stability tier; a **zero-capability** manifest is valid. Covers **AS-4, EC-004, FR-012, FR-014**.

### Implementation for User Story 1

- [ ] T010 [P] [US1] Define `Event` in `edge/host/src/bus/event.rs` — adjacently-tagged `#[serde(tag = "type", content = "data")]` enum over `Run | Goal | Vault | Voice | Ui` + `Ext { ns, name, version, payload }`, with **one representative past-tense seed variant per namespace** (e.g. `RunFinished`, `GoalAdded`, `NoteUpdated`, `UtteranceTranscribed`, `SurfaceFocused`); full leaf enumeration is deferred to `011` P0 and lands additively (FR-006). Derive `Serialize, Deserialize, schemars::JsonSchema`; `#[serde(deny_unknown_fields)]`. Covers **FR-006, FR-008, FR-009**.
- [ ] T011 [P] [US1] Define `Command` in `edge/host/src/bus/command.rs` — same namespaces + `Ext`, one **imperative** seed variant per namespace (e.g. `StartRun`, `AddGoal`); leaves deferred to `011` P0 (FR-007). Same derives + `deny_unknown_fields`. Covers **FR-007, FR-008, FR-009**.
- [ ] T012 [P] [US1] Define `ParticipantId { node: iroh::NodeId, kind: ParticipantKind, name: String, instance: Ulid }`, `ParticipantKind { GoalLoop, Agent, Connector, Scheduler, Ui, System }`, the `Agent` trait signature (`name`, `subscriptions`, async `init`/`handle`/`shutdown`), and `Subscription` (topic/namespace + filter selector) in `edge/host/src/bus/participant.rs`. Covers **FR-003, FR-011, FR-013**.
- [ ] T013 [P] [US1] Define `PluginManifest`, `Capability` (closed v1 enum, exactly 7: `Network, ProcessSpawn, VaultRead, VaultWrite, FsRead, FsWrite, SecretsRead`), and `StabilityTier { Stable, Experimental, Internal }` (new types default `Experimental`) in `edge/host/src/bus/manifest.rs`. Covers **FR-010, FR-012, FR-014**.
- [ ] T014 [US1] Define `EventId(Ulid)`, `Timestamp`, `StreamId { Run, Agent, Workspace }`, `Scope { user, workspace }`, and the `Envelope` struct (wrapping `Event` payload + `ParticipantId` origin + `StreamId` + `seq: u64` + `Scope`) in `edge/host/src/bus/envelope.rs`. Depends on T010 (Event) + T012 (ParticipantId). Covers **FR-001, FR-002, FR-004, FR-005, FR-016**.
- [ ] T015 [US1] Implement `bus::export_schemas() -> Vec<(name, serde_json::Value)>` in `edge/host/src/bus/mod.rs` (`schemars::schema_for!` per contract type), write the committed catalog to `edge/host/schemas/bus/*.json`, and make `bus_schema_validate.rs` assert committed == fresh export (drift guard; regeneration gated behind `UPDATE_SCHEMAS=1`). Depends on T010–T014. Covers **FR-015, FR-019, SC-002**.

**Checkpoint:** the US1 Independent Test passes end-to-end (round-trip + schema accept/reject + no-handle guard across all 5 namespaces + manifest); `make verify` green (nothing wired). **This is the shippable MVP.**

---

## Phase 4 — User Story 3 — Consume the contract from TypeScript (Priority: P2)

**Goal:** typed TS bindings generated from the exported schemas so Rust→TS shape drift is caught at the boundary.
**Independent Test:** generate TS bindings from the exported schemas; every `stable`-tier core type compiles against its binding and a representative payload validates.

### Tests for User Story 3 (write FIRST, ensure they FAIL)

- [ ] T016 [P] [US3] TS binding compile test in `shared/contracts/contracts.test.ts` — import every `stable`-tier core type from the generated `shared/contracts/` barrel and construct a representative value; `tsc -p shared/tsconfig.json` must compile it (RED until generation runs). Covers **SC-007, AS-1**.
- [ ] T017 [P] [US3] TS payload validation test in `shared/contracts/contracts.test.ts` — a representative payload validates against its committed `edge/host/schemas/bus/*.json` via `ajv` (already a `shared` devDep). Covers **AS-2, D-TEST-3**.

### Implementation for User Story 3

- [ ] T018 [US3] Implement the `gen:contracts` script (json-schema-to-typescript over `edge/host/schemas/bus/*.json` → `shared/contracts/*.d.ts` + `index.ts` barrel); generate the bindings; wire it into `make ts` (or a new `make contracts`). Depends on T015 (schemas must exist). Covers **FR-019, SC-007**.
- [ ] T019 [US3] Extend the dependency-direction guard so the generated `shared/contracts/` is asserted to import nothing from `platform/` (pure types only). Covers **Gate VII**.

**Checkpoint:** TS bindings generate + compile; a representative payload validates via `ajv`; Rust→TS drift fails at regen-diff, not in the frontend.

---

## Phase 5 — User Story 2 — Extend the taxonomy without changing the core (Priority: P3)

**Goal:** prove the `Ext` seam — a new event type in its own namespace validates against a plugin-registered schema with no core-enum edit.
**Independent Test:** emit an `Ext{ns,name,version,payload}` event whose payload matches its registered JSON Schema, with zero changes to the core `Event` enum source.

### Tests for User Story 2 (write FIRST, ensure they FAIL)

- [ ] T020 [P] [US2] Ext extension test in `edge/host/tests/unit/bus_schema_validate.rs` — an `Event::Ext { ns, name, version, payload }` whose `payload` matches a registered fixture schema validates; an `Ext` payload with an unexpected/oversized field is **rejected** (`additionalProperties:false`); assert the core `Event` enum source is unchanged (no new core variant). Covers **SC-006, AS-1, AS-2, EC-005**.

### Implementation for User Story 2

- [ ] T021 [US2] Add a registered extension-schema fixture `edge/host/tests/fixtures/ext/ext-slack-message.schema.json` and an `Ext`-payload resolution helper in `edge/host/src/bus/event.rs` that looks up the schema by `{ns, name, version}` from the compile-time catalog (FR-019 decision) and validates the payload. No new core `Event` variant. Covers **FR-009, SC-006**.

**Checkpoint:** an `Ext{…}` event validates against its registered schema with zero core-enum edits; all three stories pass independently.

---

## Final Phase — Polish & Cross-Cutting Concerns

- [ ] T022 [P] Module rustdoc pass on `edge/host/src/bus/mod.rs` — document the stability tiers, the additive-versioning no-break rule (FR-017), and the declared-not-enforced capability gap (sandbox deferred, `specs/012` §13.7).
- [ ] T023 Verify gates green: `make verify` (cargo test + `clippy --all-targets -- -D warnings`) + `make ts` + `make typecheck`; confirm `cmp -s CLAUDE.md AGENTS.md` still matches (no agent-guidance change in this phase).
- [ ] T024 [P] Refactor per /tdd Refactor — confirm `#[serde(deny_unknown_fields)]` on every contract type; no namespace/type uses floor-era vocabulary (D-PROJ-3); seed-variant names follow past-tense (Event) / imperative (Command).

---

## Dependencies & Execution Order

### Phase order
- Setup (T001–T003) → Foundational (T004) → US1 (T005–T015) → US3 (T016–T019) → US2 (T020–T021) → Polish (T022–T024).
- US3 depends on US1's exported schemas (T015). US2 depends on US1's `Event::Ext` variant (T010). So despite priority order, US1 must complete before US3/US2 — expected for a foundational contract (the stories layer, they don't parallelise across the US1 boundary).

### Within US1
- Tests T005–T009 precede implementation T010–T015 (Article I).
- **T005–T009 depend on T001's compat confirm** — the schemars/jsonschema/ajv tuple must hold before RED tests are written against it; so although T001 is `[P]` within Setup, it completes (with its smoke-confirm) before US1 tests begin.
- T010–T013 are `[P]` (distinct files: event/command/participant/manifest). T014 (envelope) depends on T010+T012. T015 (export) depends on T010–T014.

### Parallel opportunities
- Setup T001–T003 all `[P]`.
- US1 tests T005–T009 all `[P]`; US1 impl T010–T013 `[P]`.
- US3 tests T016–T017 `[P]`.

---

## Coverage Matrix (validator aid)

| Requirement | Task(s) |
|---|---|
| FR-001 Envelope | T014 (test T005/T006) |
| FR-002 origin+scope v1 | T014 (test T005) |
| FR-003 ParticipantId | T012 (test T005) |
| FR-004 Scope{user,workspace} | T014 (test T005) |
| FR-005 StreamId{Run,Agent,Workspace} | T014 (test T005) |
| FR-006 Event namespaces + Ext | T010 (test T005/T006) |
| FR-007 Command namespaces + Ext | T011 (test T005/T006) |
| FR-008 past-tense/imperative naming | T010/T011 (verify T024) |
| FR-009 Ext extension types | T010/T011/T021 (test T020) |
| FR-010 stability tier | T013 (test T006) |
| FR-011 Subscription shape | T012 (test T007 — NoopAgent constructs + round-trips a Subscription) |
| FR-012 PluginManifest | T013 (test T009) |
| FR-013 Agent trait | T012 (test T007 NoopAgent) |
| FR-014 Capability vocab v1 (7) | T013 (test T009) |
| FR-015 schema validation @ boundary | T015 (test T006) |
| FR-016 schema-version id | T014 (test T006) |
| FR-017 additive-versioning | T010–T014 (test T008) |
| FR-018 no embedded handle | T010/T011/T014 (test T007) |
| FR-019 catalog → TS source | T015/T018 (test T016/T017) |
| SC-001…SC-008 | T005, T006, T006, T007, T008, T020, T016, T006 |
| EC-001 size limits (minor, additive) | noted only — no hard limit set this phase |
| EC-002 stream+seq ordering | T014 (fields; enforcement `011` P1) |
| EC-003 boundary rejection | T006 |
| EC-004 empty-capability manifest valid | T009 |
| EC-005 Ext oversized field rejected | T020 |

---

## Implementation Strategy

**MVP = US1 only.** Setup → Foundational → US1 → stop, run the US1 Independent Test, and the contract is authorable and boundary-safe — everything `011` P1+ needs. US3 (TS boundary) and US2 (Ext path) are additive increments that ship after.

**Total: 24 tasks** — Setup 3, Foundational 1, US1 11, US3 4, US2 2, Polish 3. Parallel-marked: 13.
