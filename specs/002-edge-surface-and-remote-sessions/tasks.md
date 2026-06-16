# Tasks: Edge Surface & Remote Sessions

**Feature Branch:** `002-edge-surface-and-remote-sessions`
**Inputs:** [spec.md](./spec.md), [plan.md](./plan.md), `platform/docs/spec/constitution.md` v0.1.0

> **Constitution Article I (NON-NEGOTIABLE):** every behaviour-changing implementation task is preceded by a test task that exercises it. Tests are written first and confirmed failing before implementation.

## Format

`- [ ] [TaskID] [P?] [USn?] Description with concrete path` — `[P]` = parallelisable (different files, no incomplete dep); `[USn]` only on user-story tasks.

> This feature is **additive over the wedge-001 spine**. Where a task says "carried", it ports/reuses existing `apps/wagner` / wedge-001 code; the gate (`permission_server.rs` + `orchestrator/guardrails.rs`), the append-only log (`state/`), the pure reducer (`platform/shared/reducer`), and OIDC identity (ADR-0002) are reused, never reimplemented.

---

## Phase 0 — Foundation Port (added 2026-06-16; Error-Recovery per executing-plans)

> **Why this phase exists:** baseline recon (2026-06-16) found the "carried wedge-001 spine" the tasks below reuse is **not in `platform/`** — the engine (gate, log, run loop, events, memory) still lives in `apps/wagner/src-tauri/src/`, the 001 hub (OIDC + SurrealDB) was never built, and `platform/tests/architecture` is empty. Only the pure reducer + schemas are real. These preconditions are unticketed by 002 yet block Phase 2 Foundational (T009/T009a need the carried `EngineRunner` in `platform/`; T008/T013 need the OIDC-gated hub). Engineer decision 2026-06-16: **port + build all, grind 002 whole.** Reducer + schemas are reused as-is.

- [x] T000a Port the **Tauri-free engine core** (`orchestrator/`, `permission_server.rs`, `state/`, `events/`, `cli/`, `memory.rs`, `schema.rs`, `transmissions.rs`) from `apps/wagner/src-tauri/src` → `platform/edge/host/src` as a headless library; port the headless integration tests (`goal_loop`, `gate_e2e`, `permission_server`, `driver_pipe`, `eval`) + fixtures + the `gate-server.mjs` resource; `cargo test` green. The Tauri shell (`lib.rs::run`, `ipc/commands.rs`, `main.rs`) is **deferred to US1 T019/T021**. This makes the carried `EngineRunner` real in `platform/` (precondition for T009a/T016).
- [x] T000b Port the dependency-direction architecture guard → `platform/tests/architecture/dependency-direction.test.ts` (precondition for T009; Article VII).
- [x] T000c Build the **minimum-viable 001 hub** in `platform/hub` — Deno + Hono service + OIDC ID-token verification (ADR-0002: issuer/audience/signature, verified-email/group gate) + SurrealDB store skeleton (operators + the ephemeral discovery registry table). **Scope-limited** to what 002 needs; 001 recall/sync deferred. `deno test` green (precondition for T008/T013).
- [x] T000d Port the unified-surface base (`apps/wagner/src/ui` + `src/store`) → `platform/edge/ui` so a React surface folds `platform/shared/reducer`; `vitest` + `tsc` green (precondition for US1 T020).
- [x] T000e Confirm full baseline green — `cargo test` (edge/host), `vitest` (shared + edge/ui), `deno test` (hub), architecture guard — before Phase 1.

**Checkpoint:** the wedge-001 spine is real in `platform/` (engine ported + headless-test-green, MV hub up, arch guard live, surface base ported). 002's "carried" references now resolve to actual `platform/` code. Phase 1 can begin.

---

## Phase 1 — Setup (Shared Infrastructure)

- [x] T001 Add edge-host deps to `platform/edge/host/Cargo.toml` — **iroh** (Endpoint/ALPN/relay client), Tauri tray/notification features; confirm `cargo check` green (plan §Technical Context).
- [x] T002 [P] Scaffold the new host modules (empty + `mod` wiring): `platform/edge/host/src/{tray,remote}/` with `remote/{endpoint,arm,session,control,devcontext}.rs` (plan §Project Structure).
- [x] T003 [P] Scaffold `platform/shared/transport/` (the `EventStreamTransport` contract package) + `platform/edge/ui/transport/` (IPC + P2P adapter stubs) (plan §R-4).
- [x] T004 [P] Scaffold the hub discovery surface: `platform/hub/src/routes/discovery.ts` + `platform/hub/src/relay/` + the ephemeral node-discovery registry table (plan §Project Structure).

---

## Phase 2 — Foundational (Blocking Prerequisites)

No user-story work begins until this phase is complete. The transport abstraction, the new event kinds + their fold, the channel schemas, the gate-reuse seam, and remote OIDC verification underlie US1, US2, and US3.

### Tests (write FIRST, confirm FAILING)

- [x] T005 [P] Schema-validation tests for every new channel/event schema (valid sample passes; unknown field rejected via `additionalProperties:false`) in `platform/shared/schemas/remote.test.ts` (Article X, D-TEST-3; FR-005).
- [x] T006 [P] Reducer fold + **replay** test for the new event kinds — replaying a log containing `remote.armed/attached/control`, `dev_context.command/file_read/refused` reproduces a byte-identical projection; output payloads are NOT in the projection (F-1) — `platform/shared/reducer/remote-events.test.ts` (Article VIII, FR-004; SC-006).
- [x] T007 [P] Transport-abstraction unit test — a fake transport feeds an event sequence; the surface reducer produces the identical projection regardless of transport (proves FR-002 "remote feels no different") — `platform/edge/ui/transport/transport.test.ts` (FR-001/002).
- [x] T008 [P] Remote-identity contract test — an attach with no/expired/wrong-audience OIDC token is refused before any channel opens; a valid org token is accepted — `platform/hub/tests/contract/discovery_auth.test.ts` (FR-200, EC-003; reuses ADR-0002).
- [x] T009 [P] Architecture guard (carried from 001) still green: nothing outside `platform/` imports `platform/`; `shared` imports neither `edge` nor `hub` — `platform/tests/architecture/dependency-direction.test.ts` (Article VII).
- [x] T009a [P] **Edge-autonomy / offline-completion test (Article VI Gate, write FIRST):** start an edge run with the **hub unreachable** (no discovery, no relay, no hub routes — mocked 503 or no hub process) and **no remote client** attached; assert the run progresses to completion via the carried headless `EngineRunner`, and assert **zero** hub/discovery/relay calls occur on the run's execution path — `platform/edge/host/tests/integration/edge_autonomy_offline.rs` (Article VI; `constitution.md:174` "an offline-completion test exists"; plan §1.4; challenge C1). This is the foundational guarantee that remote/hub is strictly additive.

### Implementation

- [x] T010 [P] Author the channel + event JSON Schemas → `platform/shared/schemas/{remote-arm,remote-attach,remote-control,dev-context-cmd,dev-context-file,remote-event}.schema.json` (draft 2020-12, `additionalProperties:false`). Makes T005 pass (plan §1.1/1.2).
- [x] T011 [P] Add the new event-kind fold cases to the carried pure reducer + types in `platform/shared/reducer/` (no I/O; output content excluded from the projection). Makes T006 pass (Gate VIII, F-1).
- [x] T012 Define `EventStreamTransport` in `platform/shared/transport/index.ts` (`subscribe`/`send`) + the env-detected adapter selection in `platform/edge/ui/transport/`. Makes T007 pass (FR-002).
- [x] T013 Implement hub discovery/signaling routes — `POST /discovery/register` (arm; OIDC-gated, stores ephemeral `{operator_id, node_id, ticket, expires_at}`), `POST /discovery/resolve` (owner-only; 404 if not armed/owned) in `platform/hub/src/routes/discovery.ts`. Makes T008 pass (FR-201, R-3).
- [x] T014 Implement the **gate-reuse seam** in `platform/edge/host/src/remote/control.rs` — a single internal entrypoint that both the carried IPC layer and the remote control channel call into `permission_server`+`guardrails`; no second gate (FR-004, R-7). (Wired to channels in Phase 5; the seam + its unit test land here.)
- [x] T014a [P] Unit test for T014: a control action routed via the seam hits the identical gate decision as the local IPC path (parametrized over local|remote origin) — `platform/edge/host/tests/unit/gate_seam.rs` (FR-004/304, SC-202).

**Checkpoint:** Foundation ready — schemas validated, new events fold + replay, transport abstraction proven, remote OIDC gate green, single gate seam in place. User-story phases can begin.

---

## Phase 3 — User Story 1 — One surface, and a host that outlives the window (Priority: P1) 🎯 MVP

**Goal:** One responsive surface folds the host event log; closing the desktop window backgrounds to the tray with the host (and its endpoint) still running; the tray shows run status and raises needs-you notifications window-closed.
**Independent Test:** Launch a run, close the window, walk away; the run completes, the tray reflects state via glyph+label, a permission prompt raises a notification + badge within the liveness threshold, and reopening shows the live run with no loss.

### Tests (write FIRST, confirm FAILING)

- [x] T015 [P] [US1] Host lifecycle test — `WindowEvent::CloseRequested` hides (does not exit); the host **process + append-only event log** keep running after window-close; app-quit stops them — `platform/edge/host/tests/integration/tray_lifecycle.rs` (FR-101 host+log clause, FR-104, US1-AS-1/5, EC-007). *(The FR-101 remote-endpoint-survival clause is tested in US2 T025a, where the endpoint exists — challenge C2.)*
- [x] T016 [P] [US1] Run-survives-window-close test — a run executing when the window closes runs to completion (host-driven, headless `EngineRunner`); 0 interruptions — `platform/edge/host/tests/integration/run_survives_close.rs` (SC-004, US1-AS-1).
- [x] T017 [P] [US1] Tray status + needs-you test — tray state maps `idle|running|needs-you` to a non-color glyph+label; a `needs-you` transition (permission prompt / tripped guardrail) raises a notification + badge within ≤ 5 s, window-closed — `platform/edge/host/tests/integration/tray_status.rs` (FR-102/103, US1-AS-3/4, SC-008, D-A11Y-1).
- [x] T018 [P] [US1] Reopen-fidelity test — after window-closed run progress, reopening folds the host log and the projection equals the host's live snapshot (no divergence) — `platform/edge/host/tests/integration/reopen_fidelity.rs` (FR-105, US1-AS-2, Article VIII).
- [x] T018a [P] [US1] Unified-surface render test — the same UI build renders the run view from the shared reducer under both the IPC adapter and a fake P2P adapter (one codebase, env-adaptive) — `platform/edge/ui/surfaces/run_view.test.ts` (FR-001, US1-AS-6, R1).
- [x] T018b [P] [US1] Reduced-motion / contrast test — the live-activity strip has a `prefers-reduced-motion` alternative; tray + mobile status clear AA contrast and never rely on color alone — `platform/edge/ui/surfaces/a11y.test.ts` (EC-009, D-A11Y-1).

### Implementation

- [~] T019 [US1] *(logic done+tested T015-T018; live Tauri binding documented in src/tray/shell_binding.md — integration-only)* Implement tray + window lifecycle in `platform/edge/host/src/tray/` — `ActivationPolicy::Accessory`, `CloseRequested→hide`, app-quit teardown, tray icon/tooltip, native notification + badge on `needs-you` (FR-101/102/103/104). Makes T015/T016/T017 pass.
- [x] T020 [P] [US1] Port + unify the surface in `platform/edge/ui/surfaces/` — one responsive React tree (run view, browse, recall, tray-status, capability-availability) folding `platform/shared/reducer` over the transport abstraction; retire the desktop-only assumptions in the carried `apps/wagner/src/ui` (FR-001/003, R1). Makes T018a pass.
- [x] T021 [P] [US1] Wire the IPC transport adapter (`platform/edge/ui/transport/ipc.ts`) to the carried Tauri commands so the desktop surface folds the host log locally (FR-002). Makes T018 pass (local path).
- [x] T021a [P] [US1] Implement the reduced-motion + AA-contrast surface behaviours (FR-102, EC-009). Makes T018b pass.

**Checkpoint:** US1 Independent Test passes — tray-resident host, unified surface, needs-you surface end-to-end. MVP shippable (local daily driver; remote endpoint alive and ready for US2).

---

## Phase 4 — User Story 2 — Reach my running machine, and browse what the org knows (Priority: P2)

**Goal:** From a remote device, SSO-auth + edge-armed attach over iroh (direct, else org-run relay) to observe the live run; hub-side browse/recall work even with no host reachable; no self-arm.
**Independent Test:** On a second device on another network, SSO-auth and attach to an armed host across NAT, see the live run; with the host not armed/unreachable, still browse + recall; an un-armed attach is refused.

### Tests (write FIRST, confirm FAILING)

- [x] T022 [P] [US2] Arming test — `arm` is edge-only: it advertises the iroh endpoint + registers NodeId/ticket and emits `remote.armed`; there is no host API that lets a remote peer arm — `platform/edge/host/tests/integration/arming.rs` (FR-201, SC-203, US2-AS-4, EC-004).
- [x] T023 [P] [US2] Attach-over-transport test — a verified, owning client attaches over an in-memory/loopback iroh transport and folds the event stream to a live projection; relay fallback path exercised when direct is disabled — `platform/edge/host/tests/integration/attach.rs` (FR-203/210, US2-AS-1/2, EC-001).
- [x] T024 [P] [US2] No-self-arm / refused-attach test — attach to a never-armed host is refused; attach by a non-owner or unverified operator is refused; local run unaffected — `platform/edge/host/tests/integration/attach_refused.rs` (FR-212, SC-203, US2-AS-3/4, EC-003/004).
- [x] T025 [P] [US2] Ephemerality test — deliberate close tears down (re-attach requires re-arm); a transient drop **< 30 s** while armed re-attaches without re-arm, and a drop of **exactly 30 s / ≥ 30 s** requires re-arm (exclusive upper bound — test both sides of the boundary) — `platform/edge/host/tests/integration/ephemeral_session.rs` (FR-202, SC-007, US2-AS-6, EC-011).
- [x] T025a [P] [US2] **Armed-endpoint-survives-window-close test** — with a host **armed**, closing the desktop window keeps the iroh endpoint live and accepting attaches (the FR-101 remote-endpoint clause, testable only now that the endpoint exists) — `platform/edge/host/tests/integration/endpoint_survives_close.rs` (FR-101 endpoint clause, R4; challenge C2).
- [x] T025b [P] [US2] Re-arm-while-armed test — calling `arm` on an already-armed host refreshes the NodeId/ticket + expiry and emits a single new `remote.armed`, with no duplicate registration or second session — `platform/edge/host/tests/integration/rearm_idempotent.rs` (EC-010; challenge M1).
- [x] T026 [P] [US2] Hub-side-without-host test — with the host unreachable, a remote client browses learnings + runs recall; host-side capabilities show unavailable-with-reason (never silent) — `platform/edge/ui/surfaces/hubside_degraded.test.ts` (FR-204/211, SC-005, US2-AS-5, EC-002).
- [x] T026a [P] [US2] No-run-state-readable-by-hub test — during an attached session, run-state events fold over the P2P channel; assert **(a)** no hub application service (discovery/sync/store) gains a code/diff/file/transcript field (structural, schema `additionalProperties:false`) and **(b)** the relay records frame **sizes** only, never a decrypted payload — `platform/edge/host/tests/integration/no_hub_readable_runstate.rs` (FR-203, F-1, SC-006, US2-AS-7). *(Does NOT assert "zero bytes traverse the relay" — encrypted frames legitimately do; challenge H1.)*

### Implementation

- [~] T027 [US2] *(path policy + relay-frame + loopback done/tested in endpoint.rs; live iroh Endpoint wiring integration-only, ADR-0003)* Implement the iroh endpoint + per-ALPN channels + discovery-assisted connect/relay-fallback in `platform/edge/host/src/remote/endpoint.rs` (FR-203, R-1/R-2). Makes T023 pass.
- [x] T028 [US2] Implement `arm`/`disarm` (edge-only; hub discovery register/disarm; `remote.armed/disarmed` events) in `platform/edge/host/src/remote/arm.rs` (FR-201, R-3). Makes T022 pass.
- [x] T029 [US2] Implement the attach/session lifecycle — ownership + OIDC verification, attach/detach events, deliberate-close teardown, transient-drop re-attach window — in `platform/edge/host/src/remote/session.rs` (FR-202/210/212, R-3). Makes T024/T025 pass.
- [x] T030 [P] [US2] Wire the P2P transport adapter (`platform/edge/ui/transport/p2p.ts`) so the remote surface folds the attached event stream identically to local (FR-002/210). Part of T023 (UI side).
- [x] T031 [P] [US2] Surface hub-side capabilities (browse + recall) on the remote client against the carried hub, with host-side capability-availability gating + reason (FR-211/204, R5) — `platform/edge/ui/surfaces/`. Makes T026 pass.
- [x] T032 [P] [US2] Emit US2 observability — `wagner_remote_sessions_active`, `wagner_remote_attach_seconds`, `wagner_discovery_resolve_total`, `remote.attach` span — per plan §1.3.

**Checkpoint:** US2 Independent Test passes — SSO-gated, edge-armed, NAT-traversed remote observe; hub-side floor with no host; no self-arm. Reach proven.

---

## Phase 5 — User Story 3 — Act on my machine remotely, every action gated and logged (Priority: P3 — in slice one, CL-203)

**Goal:** Over the attached session, run-control ① + dev-context ② (non-interactive commands, repo-scoped file reads), every action through the same gate + onto the event log; dev-context payloads edge→device only; no PTY.
**Independent Test:** From the attached session, answer a permission, run `git diff`/`npm test`, read a file + list the tree (repo-scoped), launch a skill; each is gated, logged, attributed; no raw PTY used.

### Tests (write FIRST, confirm FAILING)

- [x] T033 [P] [US3] Run-control test — a remote `answer_permission`/`steer`/`run_skill` is delivered as the same control message the local surface sends, advances the run, and is logged attributed to the verified remote operator — `platform/edge/host/tests/integration/remote_control.rs` (FR-301, US3-AS-1/4, SC-002).
- [x] T034 [P] [US3] Gate-no-bypass test — a remote action that a guardrail would stop locally is stopped identically; remote origin cannot bypass a guardrail — `platform/edge/host/tests/integration/remote_gate_parity.rs` (FR-304, SC-202, US3-AS-5).
- [x] T035 [P] [US3] Dev-context command test — a non-interactive command streams stdout/stderr back as piped frames (no PTY allocated); the command + exit are logged, payload is not persisted to the log — `platform/edge/host/tests/integration/devctx_command.rs` (FR-302, US3-AS-2).
- [x] T036 [P] [US3] Repo-scope file-read test — an in-repo read succeeds + is logged; an out-of-repo path (incl. symlink/`..` escape, `~/.ssh`, out-of-repo `.env`) is refused + logged as `dev_context.refused` — `platform/edge/host/tests/unit/devctx_filescope.rs` (FR-303, CL-202, EC-006, SC-002).
- [x] T037 [P] [US3] Concurrent-control test — two attached clients send conflicting permission answers for the same prompt; first-write-wins through the gate, the later one is a no-op — `platform/edge/host/tests/integration/concurrent_control.rs` (CL-204, EC-005).
- [x] T037b [P] [US3] No-interactive-shell test (negative assertion) — the host registers **no PTY/interactive-shell ALPN**, and an attempt to open an interactive-shell channel is **refused with a protocol error** (not a hang/timeout); dev-context commands remain non-interactive (no PTY allocated) — `platform/edge/host/tests/integration/no_pty.rs` (FR-302, US3-AS-6, §Out of Scope ③). Proves the "③ via ssh/tmux, not built into Wagner" boundary is enforced, not merely documented.
- [x] T037a [P] [US3] Dev-context privacy test — file contents + diffs from a dev-context session reach the operator's device over P2P; assert **0** code/file/diff bytes are stored on or readable in plaintext by any hub application service, and the relay (if on-path) sees only opaque ciphertext frames (size-logged, not decrypted) — `platform/edge/host/tests/integration/devctx_privacy.rs` (FR-305, F-1, SC-006, US3-AS-7; challenge H1).

### Implementation

- [~] T038 [US3] *(gate-seam + first-write-wins done/tested T033/T034/T037; live ALPN channel binding integration-only)* Wire the control channel (ALPN `wagner/control/1`) into the T014 gate seam — run-control ① messages translated to the carried gate calls; first-write-wins serialization — `platform/edge/host/src/remote/control.rs` (FR-301/304, CL-204, R-7). Makes T033/T034/T037 pass.
- [x] T039 [US3] Implement dev-context commands (ALPN `wagner/devctx/1`) — spawn non-interactive (no PTY), pipe stdout/stderr as framed output, gate + log the invocation — `platform/edge/host/src/remote/devcontext.rs` (FR-302). Makes T035 pass.
- [x] T040 [US3] Implement repo-scoped file read + tree (canonicalize, default-deny outside repo root, `dev_context.refused` on violation) in `platform/edge/host/src/remote/devcontext.rs` (FR-303, R-6). Makes T036 pass.
- [~] T041 [P] [US3] (P2P-adapter control routing done/tested T030; carried UI components reused; live render wiring integration-only) Surface the act controls in the remote UI (answer-permission, command runner, file/tree viewer) reusing the local control components over the P2P adapter — `platform/edge/ui/surfaces/` (FR-301/302/303). Confirms no separate remote UI logic.
- [x] T042 [P] [US3] Emit US3 observability — `wagner_remote_action_total{op,outcome}`, `wagner_devctx_refused_total{reason}`, `remote.control`/`devctx.*` spans — per plan §1.3.

**Checkpoint:** US3 Independent Test passes — remote run-control + dev-context, single gate, full audit, repo-scoped reads, edge→device privacy. Slice-one (US1+US2+US3) demonstrated end-to-end.

---

## Final Phase — Polish & Cross-Cutting Concerns

- [x] T043 [P] **Author `platform/docs/adr/0003-remote-transport-iroh-org-relay.md`** — transport = iroh, org-run relay, arming/ephemeral lifecycle (recommended; parallels ADR-0001/0002). Referenced by spec §Dependencies + plan §Optional Artifacts. *(Done 2026-06-16, ahead of /execute-plan.)*
- [~] T044 [P] *(in-memory perf harness done; live SC-001 p50≤3s over NAT is integration-only)* Performance test (write FIRST): remote attach first-frame p50 ≤ 3 s over a simulated residential-NAT↔residential-NAT path with relay available — `platform/edge/host/tests/integration/attach_perf.rs` (SC-001).
- [~] T045 *(depends on live NAT path + relay — integration-only)* Performance: make T044 pass — measure attach first-frame p50 at the SC-001 conditions and tune discovery/connect if needed (SC-001).
- [x] T046 Security hardening: iroh node secret + relay config + OIDC config from env only (D-SEC-2); schema validation at every channel ingress (Article X); re-confirm the F-1 hub boundary (SC-006) and no-self-arm (SC-203).
- [x] T047 [P] `platform/specs/002-edge-surface-and-remote-sessions/quickstart.md` — arm → attach from a phone → answer a permission → `git diff` → file read, end-to-end walkthrough.
- [x] T048 [P] Docs: update `platform/README.md` (run/test edge surface + remote + relay) and `platform/prd.md` surface/remote status.
- [x] T049 Refactor per /tdd Refactor phase; confirm the gate-seam (T014a), privacy-boundary (T026a/T037a), no-self-arm (T024), and dependency-direction (T009) tests stay green.

---

## Dependencies & Execution Order

- **Setup (1)** → no deps; first.
- **Foundational (2)** → depends on Setup; **blocks** all user-story phases. Tests T005–T009a precede impl T010–T014a. The gate seam (T014/T014a) and transport abstraction (T012) are prerequisites for US1/US2/US3. **T009a (offline-completion, Article VI) is a foundational gate test** — it exercises the carried run loop with hub unreachable and must stay green through every phase.
- **US1 (3)** → depends on Foundational; tests T015–T018b precede impl T019–T021a. US1 is the independently-mergeable MVP and the precondition (surviving endpoint) for US2/US3.
- **US2 (4)** → depends on US1 (a tray-resident host to reach) + Foundational; tests T022–T026a precede impl T027–T032.
- **US3 (5)** → depends on US2 (an attached session to act through) + the T014 gate seam; tests T033–T037a precede impl T038–T042.
- **Polish (Final)** → depends on US1+US2+US3.

### Parallel Opportunities

- `[P]` tasks in a phase run concurrently (distinct files).
- Host-side (`edge/host`), surface (`edge/ui`), and hub (`hub`) tasks within a phase largely parallelise (different packages) until a channel needs both ends.
- After Foundational, US1 (tray + surface) and the US2 hub-side discovery work can split across agents; US1 is the independently-mergeable MVP.

## Implementation Strategy

1. Setup + Foundational → schemas, event fold, transport abstraction, gate seam, remote OIDC.
2. US1 → Independent Test → MVP (tray-resident host + unified surface).
3. US2 → Independent Test → remote observe + hub-side reach.
4. US3 → Independent Test → remote agency, gated + audited.
5. Polish → ADR-0003, perf (SC-001), security, quickstart, docs.
6. Hand to `/execute-plan`, which runs `/tdd` per task in order.

## Notes

- Mark `- [x]` only after acceptance is verified (the covering test passes).
- Remote/attach tests run against an in-memory/loopback iroh transport + a stubbed relay (D-TEST-4); no live relay in the suite.
- Stop at any Checkpoint to validate before continuing.
- **Coverage:** every FR maps to ≥1 test task — FR-001→T007/T018a; FR-002→T007/T012; FR-003→T026/T031; FR-004→T014a/T034; FR-005→T005; FR-100→T015; FR-101→T015 (host+log) **+ T025a (endpoint clause, US2)**; FR-102/103→T017; FR-104→T015; FR-105→T018; FR-200→T008; FR-201→T022; FR-202→T025; FR-203→T023/T026a; FR-204→T026; FR-210→T023; FR-211→T026; FR-212→T024; FR-301→T033; FR-302→T035; FR-303→T036; FR-304→T034; FR-305→T037a. Acceptance scenarios: US1-AS-1..6→T015/T016/T017/T018/T018a; US2-AS-1..7→T022/T023/T024/T025/T026/T026a; US3-AS-1..7→T033/T034/T035/T036/T037a; **US3-AS-6 (no interactive shell)→T037b** (negative assertion: no PTY ALPN; shell-open refused with a protocol error). Success criteria: SC-001→T044; SC-002→T033/T034/T036; SC-003→T024; SC-004→T016; SC-005→T026; SC-006→T006/T026a/T037a; SC-007→T025; SC-008→T017. **Article VI edge-autonomy gate → T009a (offline-completion).** Edge cases: EC-001→T023; EC-002→T026; EC-003/004→T024; EC-005→T037; EC-006→T036; EC-007→T015; EC-008→T026(+hub-down path); EC-009→T018b; **EC-010→T025b; EC-011→T025.**
