# Implementation Plan: Edge Surface & Remote Sessions

**Feature Branch:** `002-edge-surface-and-remote-sessions`
**Date:** 2026-06-15
**Spec:** [spec.md](./spec.md)
**Constitution:** `platform/docs/spec/constitution.md` v0.1.0

This plan describes HOW the edge surface + remote sessions are built. The WHAT lives in spec.md. Tasks (DO-THIS) live in tasks.md. This feature is **additive over the wedge-001 spine** (goal loop, permission + guardrail gate, append-only event log, pure reducer, OIDC identity, the Deno/SurrealDB hub) — it adds surfaces and a remote path, it does not change the engine.

## Summary

Three projections of the existing event-sourced run, plus the path between them:

1. **Unify the surface (R1).** The carried desktop-only TS UI (`apps/wagner/src/ui`) becomes one responsive React/TS codebase rendered in a Tauri desktop shell, a browser, and on mobile. Run state is folded by the **same pure reducer** (`platform/shared/reducer`) in every environment; only the **transport** below the reducer differs (in-process IPC for desktop, P2P for remote). The reducer never knows which transport delivered the event, so a remote surface renders the identical projection a local one does (FR-001/002); remote attach latency is bounded separately by SC-001 (first-frame p50 ≤ 3 s).
2. **Make the host tray-resident (R3/R4/R6).** The host is embedded in the Tauri desktop app (carried `apps/wagner/src-tauri`, ported to `platform/edge/host`). Closing the window backgrounds the app to the macOS menu bar via `ActivationPolicy::Accessory` + a `WindowEvent::CloseRequested` → hide (not exit) handler, so the host, its event log, and its iroh endpoint survive window-close. The tray projects run status (`idle | running | needs-you`) with a non-color glyph + label and raises native notifications + a badge on `needs-you`. **This is new host work** — the carried `Tray.tsx` is a React mock; no real Tauri tray / window-lifecycle wiring exists yet (verified: no `ActivationPolicy`/`CloseRequested`/tray refs in `apps/wagner/src-tauri`).
3. **Add remote sessions with agency (D3/D4/D5/D6).** A remote client authenticates via the carried OIDC identity (ADR-0002), and — after the operator **arms** the host on the edge machine — discovers and attaches to it over **iroh** (QUIC, Ed25519 node identity, NAT traversal) with an **org-run relay** fallback (CL-201). Over the attached session the operator runs the same control messages (run-control ①) and non-interactive dev-context ② (commands, repo-scoped file reads — CL-202), every action routed through the **same gate** (`permission_server.rs` + `guardrails.rs`) and appended to the same event log (D6). The hub's role is strictly **discovery / signaling / relay-coordination + identity** — never execution or plaintext run-state relay (Article VI; F-1).

Slice-one scope (CL-203): **US1 + US2 + full US3** (observe **and** act); fleet + presence deferred.

---

## Technical Context

| Field | Value |
|-------|-------|
| Language / Version | **Edge host:** Rust (carried `apps/wagner/src-tauri`; rustc 1.90 / edition 2021 as carried — note wedge-001 plan said 1.87/2024, carried reality is 1.90/2021). **Edge UI:** TypeScript 5.x + React (carried `apps/wagner/src/ui`). **Hub:** TypeScript on Deno (ADR-0001). |
| Primary Dependencies | **Edge host:** Tauri (tray, window lifecycle, notifications), **iroh** (QUIC P2P endpoint, Ed25519 node identity, NAT traversal, relay client — NEW), the carried gate (`permission_server.rs`, `orchestrator/guardrails.rs`), `serde`/schema validation. **Edge UI:** React, the shared pure reducer, a transport abstraction (IPC vs P2P). **Hub:** `hono`, `surrealdb` JS SDK, `ajv`, the OIDC client (carried), an **iroh discovery/signaling + org-run relay** surface (NEW). |
| Storage | **Edge:** the carried append-only event log + run state under `.wagner/`; the remote/arming/dev-context actions are **new event kinds** appended to the same log (no new store). **Hub:** carried SurrealDB (operators, enrollments, runs, learnings) + a small **node-discovery registry** (operator → armed host iroh `NodeId` + signaling ticket; ephemeral). |
| Testing | `cargo test` (edge host — gate, tray lifecycle via headless harness, iroh attach over a loopback/in-memory transport — D-TEST-1/4); `vitest` (surface reducer + transport abstraction, env-adaptive render — D-TEST-2); `deno test` (hub discovery/signaling routes); schema-validation tests for every channel message (D-TEST-3); a stubbed/in-memory relay for remote tests (no live relay in the suite — D-TEST-4); `bats` (packaging — D-TEST-5). |
| Target Platform | **Edge desktop:** macOS primary (carried; tray via `ActivationPolicy::Accessory`). **Remote client:** any modern browser + mobile web (responsive). **Hub + relay:** Linux (Deno service + SurrealDB + an iroh relay process). Native Windows/Linux desktop deferred (CL-205 — web client satisfies cross-OS reach). |
| Project Type | Multi-package within `platform/`: `shared/` (schemas + reducer + transport contract), `edge/` (host + ui), `hub/` (service + discovery/signaling + relay). |
| Performance Goals | Remote attach first-frame p50 ≤ 3 s over residential-NAT↔residential-NAT with relay available (SC-001, provisional); needs-you notification ≤ 5 s window-closed (SC-008); run completion unaffected by window-close (SC-004). |
| Constraints | Edge offline/local-capable (Article VI — remote is additive, never on the run critical path); no metered LLM API key (D-SEC-1, carried); **only metadata + learnings cross the edge→hub boundary, never run-bearing content even during a remote session** (Article IX; F-1); every channel message schema-validated (Article X); nothing outside `platform/` imports `platform/` (Article VII). |
| Scale / Scope | Single trusted org: tens of operators, a handful of concurrent remote sessions per host. iroh + one org-run relay covers this comfortably; no horizontal scaling in this slice. |

> No remaining Technical-Context unknowns block the plan. The five spec clarifications (CL-201..205) are resolved; provisional SC targets (SC-001/007/008) are confirmed by the perf tasks in tasks.md.

---

## Constitution Check

Gates refer to `platform/docs/spec/constitution.md` v0.1.0 (Articles I–X). Failed/added items have Complexity Tracking entries below.

- [x] **Gate I — Test-First:** tasks.md lists every test task before the implementation task it covers (tray lifecycle, arming, attach, gate-reuse, repo-scope guard, privacy-boundary).
- [x] **Gate II — Evidence-Driven:** quantified or cited throughout. The cross-environment-parity claim is stated mechanically (FR-001/002: identical reducer over an abstracted transport), not as an experiential adjective; latency claims are in SC-001/007/008 with fixed conditions. *(Challenge H2: the earlier "remote feels no different" plan-prose was removed — an unquantified experiential phrase fails Article II even when the mechanism behind it is sound.)*
- [x] **Gate III — CRITICAL-Resolved:** no open CRITICAL at plan time.
- [x] **Gate IV — Independent MVP:** US1 (tray-resident host + unified surface) has a concrete, non-empty Independent Test and ships value without US2/US3.
- [⚠] **Gate V — Simplicity:** this feature adds **iroh** (a P2P transport dependency), an **org-run relay** (a new deploy/operate surface), a **discovery/signaling registry** on the hub, and **real Tauri tray/window-lifecycle wiring**. Each has a Complexity Tracking entry below; all are irreducible given the spec (remote agency *is* the feature) or explicit engineer decisions (CL-201; design.md D3).
- [x] **Gate VI — Edge-Executes-Hub-Remembers:** the run's critical path touches neither hub nor remote synchronously (see §1.4). Remote is additive: a remote action is a control message into the **already-local** gate; the hub does discovery/signaling/relay only and **never executes**; the relay carries only opaque encrypted frames (F-1). **Offline-completion test = T009a** (start + complete an edge run with the hub unreachable and no remote client; assert zero hub/discovery/relay calls on the run path) — satisfies `constitution.md:174` (challenge C1).
- [x] **Gate VII — One-Directional Dependency:** `edge`/`hub` depend on `shared`, never the reverse; nothing outside `platform/` imports it. The unified surface lives in `platform/edge/ui`, importing `platform/shared/reducer`. Dependency-direction test carried from 001 stays green.
- [x] **Gate VIII — Event-Sourced Truth:** every remote/lifecycle action is a **new event kind** folded by the pure reducer; the surface (local or remote) is a projection of the log. Replay-equals-snapshot extended to the new event kinds. No run-state mutation outside the fold.
- [x] **Gate IX — Privacy-Boundary:** unchanged and reinforced by **F-1** — remote dev-context (code/diffs/files) flows edge→operator-device over P2P, **never** to the hub; the hub-bound sync payload schema is still metadata + learnings only (`additionalProperties:false`). A test asserts zero run-bearing bytes to the hub during a remote dev-context session (SC-006).
- [x] **Gate X — Schema-Validated:** every channel message (arm, attach, control, dev-context request/response) validates against a declared schema before it is acted on; CI validates committed schemas + sample payloads.

---

## Project Structure

```
platform/
├── shared/
│   ├── schemas/                      # + NEW channel schemas (draft 2020-12, additionalProperties:false)
│   │   ├── remote-arm.schema.json         # NEW: edge arm action + signaling ticket
│   │   ├── remote-attach.schema.json      # NEW: client attach handshake (SSO subject + node addr)
│   │   ├── remote-control.schema.json     # NEW: run-control ① message (steer / answer-permission / run-skill)
│   │   ├── dev-context-cmd.schema.json    # NEW: non-interactive command request + streamed-output frames
│   │   ├── dev-context-file.schema.json   # NEW: file-read / tree request + response (repo-scoped)
│   │   └── remote-event.schema.json       # NEW: the new event kinds (arm/attach/detach/remote-control/dev-context-*)
│   ├── reducer/                      # carried pure reducer; + fold cases for the NEW event kinds
│   └── transport/                    # NEW: transport contract (EventStreamTransport: IPC | P2P) the surface folds over
├── edge/
│   ├── host/                         # Rust (Tauri) host (ported from apps/wagner/src-tauri)
│   │   ├── src/
│   │   │   ├── orchestrator/         # carried goal loop + guardrails (the gate)
│   │   │   ├── permission/           # carried permission_server (the gate's permission half)
│   │   │   ├── state/                # carried append-only log; + append the NEW event kinds
│   │   │   ├── tray/                 # NEW: ActivationPolicy::Accessory, CloseRequested→hide, status glyph, notifications, badge
│   │   │   ├── remote/               # NEW: iroh endpoint, arming, attach/session lifecycle, capability channels
│   │   │   │   ├── endpoint.rs            # iroh Endpoint + ALPN + discovery via hub
│   │   │   │   ├── arm.rs                 # edge-side arm/disarm; no remote self-arm (FR-201)
│   │   │   │   ├── session.rs             # ephemeral attach/detach + transient-drop re-attach (FR-202)
│   │   │   │   ├── control.rs             # run-control ① → into the SAME gate (FR-004/301)
│   │   │   │   └── devcontext.rs          # dev-context ②: piped cmd + repo-scoped file read (FR-302/303)
│   │   │   └── ipc/                  # carried Tauri commands (local transport) + the transport abstraction seam
│   │   └── tests/{unit,integration}/
│   └── ui/                           # ONE responsive React/TS surface (desktop/web/mobile)
│       ├── transport/                # IPC adapter (Tauri) + P2P adapter (remote) implementing shared/transport
│       ├── surfaces/                 # responsive run view, browse, recall, tray-status, capability-availability
│       └── *.test.ts
└── hub/                              # carried Deno + Hono + SurrealDB; + NEW discovery/signaling + relay
    ├── src/
    │   ├── routes/                   # carried /auth /projects /runs /learnings /recall
    │   │   └── discovery.ts               # NEW: register armed host NodeId; resolve for a verified peer; signaling
    │   ├── relay/                    # NEW: org-run iroh relay coordination (or a standalone relay process)
    │   └── store/                    # + ephemeral node-discovery registry
    └── tests/{contract,integration,unit}/
```

**Structure Decision:** Extends the wedge-001 three-package tree (`edge`,`hub` → `shared`, never the reverse — Gate VII). The **transport contract** lives in `shared/transport` so the surface depends on an abstraction, not on Tauri-IPC or iroh directly (FR-002). The host gains two new sibling modules under `edge/host/src` — `tray/` and `remote/` — keeping the carried `orchestrator`/`permission`/`state` untouched as the gate + log the remote path routes **through**, not around (Gate VI, D6). The hub gains a `discovery` route + a `relay` surface; both are bounded to signaling/relay and never touch run execution (Article VI; F-1). This is the smallest layout that adds the two surfaces + the remote path without a new project (Gate V).

---

## Phase 0 — Research

Inlined (Decision / Rationale / Alternatives). A standalone `research.md` can be extracted on request.

- **R-1 — Remote transport = iroh (QUIC P2P).** *(engineer decision, design.md D3)*
  - **Decision:** iroh `Endpoint` with an app-defined ALPN per capability channel; Ed25519 node identity (authenticated peers); QUIC encryption end-to-end; NAT traversal built in. The run stays on the edge (model a); the remote client reaches in.
  - **Rationale:** iroh is purpose-built for authenticated, encrypted, NAT-traversed app-defined streams ("Tailscale for app-defined streams" — design.md D3); it gives per-channel ALPNs that map cleanly onto the capability tiers (control vs dev-context), and E2E encryption is what lets the relay stay zero-knowledge (F-1).
  - **Alternatives:** raw WebSocket-through-the-hub (forces run-state-bearing content through the hub — fails Article VI/IX/F-1); WebRTC data channels (heavier signaling, browser-centric, weaker server-side identity story); SSH tunnel as the primary (that is tier ③, explicitly the thing we are *not* rebuilding — D4).

- **R-2 — Relay = org-run (CL-201).**
  - **Decision:** Stand up an org-operated iroh relay; the edge host and remote client use it for discovery-assisted hole-punching and as a fallback path when no direct connection forms. The relay only ever forwards opaque encrypted QUIC frames.
  - **Rationale:** Engineer chose to keep **connection metadata** in-house (CL-201); payloads are E2E-encrypted regardless (F-1), so the relay is zero-knowledge of content; one relay covers tens of operators. Discovery (NodeId resolution + signaling) is folded into the existing hub, so the only *new* process is the relay itself.
  - **Alternatives:** iroh public relays (lower ops, but connection metadata transits a third party — rejected per CL-201); no relay / direct-only (fails behind symmetric NAT — EC-001 would have no fallback).

- **R-3 — Arming + attach + ephemeral session lifecycle (FR-201/202, D5).**
  - **Decision:** `arm` is an **edge-only** host action that (a) starts/advertises the iroh endpoint, (b) registers the host `NodeId` + a short-lived signaling ticket in the hub discovery registry under the operator's verified identity, and (c) records a `remote.armed` event. A remote client, after OIDC verification, resolves the ticket and attaches; `attach`/`detach` are `remote.attached`/`remote.detached` events. **Deliberate close** removes the discovery registration + tears down the endpoint advertisement (must re-arm). A **transient drop while still armed** keeps the registration alive for a re-attach window (≤ 30 s, SC-007), after which it lapses.
  - **Rationale:** Edge-only arming makes self-arm structurally impossible (FR-201, SC-203); routing arm/attach/detach through the event log gives a free audit trail and makes the session a projection like everything else (Gate VIII).
  - **Alternatives:** always-listening host (no arm gate — violates D5 opt-in, enlarges attack surface); hub-stored long-lived tokens (a credential authority on the hub — rejected, mirrors the 001 R-1 rejection).

- **R-4 — Unified surface via a transport abstraction (FR-001/002, R1).**
  - **Decision:** Define `EventStreamTransport` in `shared/transport`: a minimal interface (`subscribe(events) / send(message)`) with two adapters — an **IPC adapter** (Tauri commands/events, desktop) and a **P2P adapter** (iroh channel, web/mobile). The React surface depends only on the interface + the shared reducer; environment detection picks the adapter at boot.
  - **Rationale:** One codebase, three environments (R1) requires the surface to be transport-blind; the reducer already has zero I/O dependency (Article VIII), so the seam is exactly at message delivery.
  - **Alternatives:** separate desktop and web builds (two codebases — the thing R1 rejects); push everything through the Tauri webview only (no browser/mobile reach).

- **R-5 — Tray-resident host lifecycle (FR-100..105, R4/R6).**
  - **Decision:** macOS `ActivationPolicy::Accessory` so the app lives in the menu bar; intercept `WindowEvent::CloseRequested` to `hide()` the window instead of exiting; a tray icon whose template image + tooltip encode `idle | running | needs-you` (glyph + label, D-A11Y-1); native notifications + a dock/tray badge raised on the `needs-you` transition. App-quit (menu/`Cmd-Q`) is the explicit teardown (FR-104).
  - **Rationale:** This is the carried "never looks frozen" + "supervise without babysitting" contract (`apps/wagner/PRODUCT.md` principles 3/4) made real; it is also the precondition for any remote endpoint to survive window-close.
  - **Alternatives:** a separate daemon now (R4 defers this — would be a rewrite of packaging before the surface is proven); leave the host tied to the window (ends runs on close — the problem we are fixing).

- **R-6 — Repo-scoped file-read guard (FR-303, CL-202, EC-006).**
  - **Decision:** Every dev-context file read/tree request is canonicalized (resolve symlinks + `..`) and checked against the run's repo root; outside-root → refused + a `dev_context.refused` event. Default-deny.
  - **Rationale:** Repo-scope makes out-of-repo secrets (`~/.ssh`, `~/.aws`, out-of-repo `.env`) unreachable by construction (CL-202), a stronger guarantee than a denylist.
  - **Alternatives:** any-path + protected denylist (default-allow; must keep the denylist correct — rejected per CL-202); no guard (unacceptable — a remote file-read primitive with no scope).

- **R-7 — Remote actions reuse the carried gate (FR-004/304, D6).**
  - **Decision:** Run-control and dev-context messages are translated, at the host edge of the `remote/` module, into the **same** internal calls the local IPC layer makes into `permission_server` + `guardrails`. There is no second gate. Concurrent control is **first-write-wins**: the gate serializes; a later conflicting decision for the same prompt is a no-op (CL-204).
  - **Rationale:** A single chokepoint is the only way to guarantee a remote action can't bypass a guardrail (SC-202) and that the audit trail is complete (SC-002).
  - **Alternatives:** a remote-specific permission path (two gates → drift, the bypass risk the spec forbids).

---

## Phase 1 — Design & Contracts

### 1.1 New event kinds (appended to the carried log; folded by the pure reducer)

All validate against `remote-event.schema.json` (Article X) and fold via the carried reducer (Gate VIII):

- `remote.armed { operator_id, node_id, ticket_id, ts }` / `remote.disarmed { ts }`
- `remote.attached { operator_id, client_id, ts }` / `remote.detached { client_id, reason: closed|dropped, ts }`
- `remote.control { client_id, kind: steer|answer_permission|run_skill, ref, ts }` — the audited record of a run-control ① action (the *effect* is the carried run/permission event it triggers)
- `dev_context.command { client_id, argv, cwd, ts }` + streamed `dev_context.output { stream: stdout|stderr, chunk_seq }` (output frames are transient transport, the command + exit are logged)
- `dev_context.file_read { client_id, path, bytes, ts }` / `dev_context.tree { client_id, root, ts }` / `dev_context.refused { client_id, path, reason: out_of_scope, ts }`

These make every remote action a first-class, replayable, attributable event (SC-002). Output *content* (stdout/stderr/file bytes) streams over the P2P channel to the operator's device and is **not** persisted to the log or synced to the hub (F-1, SC-006) — the log records that the action happened + its metadata, not the payload.

### 1.2 Channel contracts (iroh, per-tier ALPN; all bodies schema-validated, Article X)

- **Arm (edge-local, no channel)** → starts the iroh endpoint, registers `node_id` + signaling ticket in the hub discovery registry under the verified `operator_id` (FR-201). `remote-arm.schema.json`.
- **Discovery (hub, OIDC-gated)** → `POST /discovery/resolve` returns an armed host's reachable address/ticket for a verified peer who owns it; `404` if not armed / not owned. Hub does signaling only — no run data. `remote-attach.schema.json`.
- **Attach channel** (ALPN `wagner/attach/1`) → client presents the OIDC subject + ticket; host verifies ownership, opens the event stream, emits `remote.attached`. Ephemeral; transient-drop re-attach within the SC-007 window (FR-202).
- **Control channel** (ALPN `wagner/control/1`) → `remote-control.schema.json`; each message is translated into the carried gate calls (R-7); first-write-wins (CL-204).
- **Dev-context channel** (ALPN `wagner/devctx/1`) → `dev-context-cmd.schema.json` (non-interactive argv, streamed output frames) + `dev-context-file.schema.json` (repo-scoped read/tree). No PTY.

Idempotency / ordering: control messages carry a client-assigned monotonic seq so the gate can drop duplicates on re-attach. Versioning: ALPN strings carry a version suffix; schemas carry a `schema` const (D-STORE-1).

### 1.3 Cross-Cutting Concerns

#### Observability
- **Log fields (host remote + tray):** `operator_id`, `client_id`, `op` (`arm`|`attach`|`detach`|`control`|`devctx_cmd`|`devctx_file`), `transport` (`direct`|`relay`), `outcome`, `duration_ms`, `scope_ok` (bool, file reads), `schema_valid` (bool).
- **Metrics (host):** `wagner_remote_sessions_active`, `wagner_remote_attach_seconds` (histogram, SC-001), `wagner_remote_action_total{op,outcome}`, `wagner_devctx_refused_total{reason}`, `wagner_tray_needsyou_total`. **Hub:** `wagner_discovery_resolve_total{outcome}`, `wagner_relay_sessions_active`.
- **Trace spans:** `remote.attach` (discovery→connect→first-frame), `remote.control` (per action, child of the run), `devctx.cmd` / `devctx.file`.

#### Security
- **Trust boundary:** the iroh channel (edge↔client) and the hub discovery/signaling endpoint. Peers are authenticated by Ed25519 node identity **and** OIDC operator identity; a channel opens only after both (FR-200, FR-203). Every inbound message schema-validated before action (Article X).
- **Authentication / Authorisation:** OIDC verifies the remote operator (ADR-0002, carried). A client may attach only to a host **it owns** (same `operator_id`) and only if **armed** (FR-201). Dev-context file reads are repo-scoped default-deny (R-6). Arming is edge-only (no self-arm — SC-203).
- **Secrets:** iroh node secret key, relay config, OIDC config, hub session secret from environment (D-SEC-2); never inlined. Edge sets no LLM API key (D-SEC-1).
- **Privacy (F-1):** the relay is zero-knowledge (opaque encrypted frames); the hub stores only an ephemeral NodeId/ticket for discovery, never run-bearing content; dev-context payloads are edge→operator-device only (SC-006). No `shared/schemas` sync payload gains a code/diff/file field — the boundary stays structural.

#### Failure Modes
- **No direct path + relay reachable** → attach over relay (EC-001). **No path at all** → attach fails with a clear reason; **local run unaffected** (Article VI).
- **Host lost mid-session** → client drops host-side capabilities to unavailable-with-reason, **keeps** hub-side (browse/recall); re-attaches when the host returns and the session is still armed (EC-002).
- **Hub/discovery down** → no **new** attach + hub-side empty/unavailable; an **already-established** direct P2P session continues; local run unaffected (EC-008).
- **Transient drop ≤ 30 s while armed** → silent re-attach (SC-007); **deliberate close** → teardown, must re-arm.
- **App quit** → host + endpoint stop; attached clients drop to hub-side only (EC-007).

### 1.4 Edge-run critical path with remote present (Gate VI proof)

Remote adds **zero** synchronous hub or remote calls to the run's critical path. A remote action is a message into the **already-local** gate; arming/attach are out-of-band; the hub does discovery/signaling only.

```
run start
  └─ goal loop executes locally on subscription CLIs        [CRITICAL PATH — no hub, no remote call]
       └─ events fold into local append-only log (pure reducer)
       └─ a permission prompt blocks on the LOCAL gate       [answered by local IPC OR a remote.control msg — same gate]
  └─ (out of band) operator arms host → register NodeId/ticket on hub  [discovery only; not on run path]
  └─ (out of band) remote client: OIDC → resolve ticket → iroh attach   [direct, else org-run relay; not on run path]
       └─ remote control/dev-context msgs ──► the SAME gate ──► fold to log  [gated + audited; never bypasses]
       └─ dev-context payloads stream edge→operator-device over P2P          [never to the hub — F-1, SC-006]
```

A host with no armed session, no remote client, or no hub runs identically — remote is strictly additive (Article VI). The offline/remote-absent completion test (tasks) and the no-self-arm test (SC-203) assert this.

---

## Complexity Tracking

| Violated Gate | Why Needed | Simpler Alternative Rejected Because |
|---------------|------------|--------------------------------------|
| Gate V — **iroh P2P transport** (R-1; design.md D3) | Remote *agency* with the run staying on the edge requires an authenticated, encrypted, NAT-traversed app-defined channel; iroh is purpose-built for it and its per-ALPN streams map onto the capability tiers. | WebSocket-through-hub: forces run-bearing content through the hub (fails Article VI/IX/F-1). Direct-only: no NAT fallback. |
| Gate V — **org-run relay** (R-2; CL-201) | A fallback path for symmetric-NAT peers (EC-001) that keeps connection metadata in-house (engineer decision CL-201); zero-knowledge of content (F-1). | Public relays: metadata transits a third party (rejected per CL-201). No relay: no fallback behind hard NAT. |
| Gate V — **hub discovery/signaling registry** (R-3) | A verified peer must resolve an armed host's reachable address; the smallest possible addition is an ephemeral NodeId/ticket registry on the existing hub. | A standalone discovery service: a second service for a few rows. Hub-stored long-lived tokens: a credential authority (rejected, mirrors 001 R-1). |
| Gate V — **real Tauri tray/window-lifecycle wiring** (R-5) | The host must survive window-close to be a remote endpoint and to keep long runs alive; this is new (the carried `Tray.tsx` is a UI mock, no lifecycle wiring exists). | Daemon now: a packaging rewrite before the surface is proven (R4 defers it). Window-tied host: ends runs on close (the bug we fix). |

All additions are bounded: iroh + one relay + an ephemeral registry on the existing hub + tray wiring on the existing app. None introduces a queue broker, Temporal, or a second datastore. The relay (one process) is the only new operational surface; the org already operates the IdP and the hub.

---

## Optional Artifacts

Created on request, not by default:

- [x] **`docs/adr/0003-remote-transport-iroh-org-relay.md`** — **recommended**: record transport = iroh + org-run relay + the arming/ephemeral lifecycle (parallels ADR-0001/0002). Strongly advised given it is a new load-bearing dependency. *(Authored 2026-06-16 — T043 done ahead of execution.)*
- [ ] `data-model.md` — the ephemeral discovery registry + new event-kind fields (summarized §1.1/1.2).
- [ ] `contracts/*` — the channel ALPNs + message schemas (summarized §1.2; the schemas themselves are authored as tasks).
- [ ] `research.md` — Phase 0 findings (inlined above).
- [ ] `quickstart.md` — arm → attach from a phone → answer a permission → run `git diff` → read a file, end-to-end.
