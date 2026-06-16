# Feature Specification: Edge Surface & Remote Sessions

**Feature Branch:** `002-edge-surface-and-remote-sessions`
**Created:** 2026-06-15
**Status:** Draft (clarified — Session 2026-06-15)
**Input:** `/brainstorm` output `design.md` §Resolved (R1–R6, 2026-06-15 engineer design session) + handoff 2026-06-15: "give the operator one GUI surface across desktop/web/mobile, keep the run executing on the operator's machine, and let a remote client *act* (run a skill, run a command, see diffs, read files) — not just observe like Claude Code's remote does."

> **Spec authoring rules** (from `platform/docs/spec/constitution.md`):
> - Focus on **WHAT** users need and **WHY**. No HOW (transport library, tray API, framework) — that lives in plan.md.
> - Every requirement is testable; vague adjectives without adjacent quantification fail Article II.
> - User stories are P1/P2/P3 and each P1 is independently shippable (Article IV).
> - No silent defaults: catalog defaults are cited in `## Defaults Applied`; everything else the engineer didn't specify is `[NEEDS CLARIFICATION]`. No marker budget — `/spec clarify` resolves them in batches of 5.

---

## User Scenarios & Testing *(mandatory)*

This feature changes **where and how an operator drives a run** without changing the run engine. The engine is already headless and event-sourced (wedge 001: the host folds an append-only event log with a pure reducer, zero UI dependency — Article VIII). The surface was never the engine; it is a thin projection of the event stream, and several projections can coexist (design.md D1). This feature builds three of them and the path between them:

1. a **unified GUI surface** — one React/TS codebase rendered in a desktop (Tauri) shell, a browser, and on mobile (responsive) — replacing per-environment surfaces (R1);
2. a **tray-resident native host** — the host embeds in the Tauri desktop app and backgrounds to the menu bar / system tray so the run and its remote endpoint survive window-close (R3, R4, R6);
3. **remote sessions with agency** — a remote web/mobile client reaches *into* the operator's running machine over an authenticated P2P channel and can *act* (run a skill, run a command, read files, see diffs), not merely observe (D3, D5, D6).

Capability availability is a function of **host-reachability** (R5): hub-side capabilities (browse shared memory, recall, fleet, presence) need no host and work on any surface; host-side capabilities (run-control ①, dev-context ②) require a reachable host over the P2P channel. The three stories below are the local foundation (US1), reach + observe + hub-side capability (US2), and remote agency (US3).

> **Slice-one scope (CL-203, resolved 2026-06-15 — "full agency").** The first shippable slice is **US1 + US2 + the full US3** (remote observe **and** remote act: run-control ① and dev-context ②). **Fleet** (multi-run view) and **presence** (who-is-running) are **deferred** to a follow-up slice. Priorities below order the stories by build-dependency and independent-shippability (US1 is the P1 MVP per Article IV); the slice-one **definition of done includes all three stories** (mirrors wedge-001 CL-1, where US1=P1 but the wedge shipped US1+US2 together).

### User Story 1 — One surface, and a host that outlives the window (Priority: P1)

An operator opens Wagner on their desktop and supervises a long, message-sparse autonomous run through a single GUI surface. They close the window to get it out of the way. The app does **not** quit — it backgrounds to the menu bar / system tray, the run keeps executing, and the tray icon reports the run's state at a glance (idle / running / needs-you). When the run hits a permission prompt or a guardrail while the window is closed, the tray raises a native notification and a badge. The operator reopens the window and immediately sees the live run, exactly where it is. The same surface — one codebase — is what will later render in a browser and on mobile (US2), but here it runs locally against the host over the desktop's in-process channel.

**Why this priority:** This is the load-bearing foundation. Until the host can survive window-close as a tray-resident process, there is no live endpoint for a remote client to reach (US2/US3 are impossible without it), and the operator cannot "supervise a fleet while doing other things" (`apps/wagner/PRODUCT.md` §Users) because closing the window today ends the run. Shipping only US1 delivers a real, usable outcome — a tray-resident daily driver that keeps long runs alive and surfaces needs-you state without a window — and it is the precondition every remote capability builds on. It is the independently-shippable MVP (Article IV).

**Independent Test:** An operator launches a coding run, closes the desktop window, and walks away. Verify that (a) the run continues to completion with the window closed (the host process and its event log keep running), (b) the tray icon reflects the run's current state via a non-color glyph + label (idle / running / needs-you), (c) when the run raises a permission prompt with the window closed, a native notification + tray badge appear within the liveness threshold (SC-008), and (d) reopening the window shows the live run state folded from the host's event log with no loss — confirming the unified surface, the tray-resident host lifecycle, and the needs-you surface end-to-end, with no remote path required.

**Acceptance Scenarios:**

1. **Given** a run executing in the desktop app, **When** the operator closes the window, **Then** the app backgrounds to the menu bar / system tray (it does not exit), and the host process, its append-only event log, and its remote endpoint keep running (R4).
2. **Given** a run executing with the window closed, **When** the operator reopens the window, **Then** the surface folds the host's event log via the pure reducer and shows the current run state with no divergence from what the host holds (Article VIII; design.md D1).
3. **Given** a run executing with the window closed, **When** the run reaches a `needs-you` state (a permission prompt or a tripped guardrail), **Then** the tray raises a native OS notification and shows a badge, and the tray icon's status glyph changes to the `needs-you` state (R6; `apps/wagner/PRODUCT.md` principle 3).
4. **Given** the tray is showing run status, **When** any state is displayed (idle / running / needs-you), **Then** that state is conveyed by a non-color glyph (e.g. `○ ◉ ✕`) **and** a text label, never by color alone (D-A11Y-1; `apps/wagner/PRODUCT.md` §Accessibility).
5. **Given** the operator quits the app entirely (not just closing the window), **When** the app exits, **Then** the host process and its remote endpoint stop — this is the intended boundary between "window closed, host alive" and "app quit, host stopped" (R4; EC-007).
6. **Given** the unified surface renders the run, **When** the same build is loaded as a web client (US2 transport), **Then** it renders the identical run view from the identical reducer — the surface is one codebase, not a desktop-only build (R1).

---

### User Story 2 — Reach my running machine, and browse what the org knows (Priority: P2)

An operator is away from their desk. From their phone (or any browser on another network), they open Wagner, authenticate with the org SSO, and — because they explicitly **armed** the session on their desk machine earlier — discover and attach to their still-running host across the network (including across NAT). They now see the live run exactly as they would locally. Even before attaching to a host (or when their machine is asleep and unreachable), they can still **browse the org's shared memory and recall** — the hub-side capabilities that need no host — so the trip is never wasted. The remote attachment is **ephemeral**: if they deliberately close it, the tunnel tears down and the desk machine must be re-armed to reconnect; a transient network drop while still armed lets them re-attach without re-arming.

**Why this priority:** Remote *observation* (plus the always-available hub-side capabilities) is the reach payoff and the precondition for remote *action* (US3) — an operator must be able to authenticate, discover, and attach to their host before they can act through it. It is sequenced after US1 (there is no surviving endpoint to reach without the tray-resident host) and before US3 (acting requires an attached, observing session first). It is separable: US2 ships a remote observe + org-memory client that is already "better than nothing from a phone," without the host-side agency of US3. The hub-side capabilities also degrade gracefully — they are the floor a remote client always offers even with no reachable host (R5).

**Independent Test:** On a second device on a different network, an operator authenticates via org SSO, and — given the host was armed on the desk machine — discovers and attaches to it across NAT and sees the live run state fold in (identical to local). Separately, with the desk machine **not** armed / unreachable, the same remote client still browses the org's shared learnings and runs recall (hub-side), and an attempt to attach to an un-armed host is **refused** — confirming SSO-gated reach, edge-armed attachment, NAT traversal, host-independent hub-side capability, and the no-self-arm guarantee.

**Acceptance Scenarios:**

1. **Given** a host armed on the desk machine and a verified remote operator, **When** the remote client discovers and attaches over the P2P channel, **Then** it folds the host's event stream and renders the live run identically to the local surface (design.md D1, D3).
2. **Given** a remote client and a host on networks behind NAT, **When** direct P2P connection is not possible, **Then** the connection falls back to a relay and still attaches; if neither direct nor relay path is available, attach fails gracefully and the run continues locally unaffected (D3; EC-001; Article VI).
3. **Given** a remote operator without a valid org SSO session, **When** they attempt any remote action, **Then** the attachment is refused before any channel opens and the run is unaffected (D5; FR-203; EC-003).
4. **Given** a desk machine on which the host was **never armed** for a remote session, **When** a remote client attempts to attach, **Then** the attachment is refused — a remote client can never bootstrap its own access (D5; SC-203; EC-004).
5. **Given** a remote client with the host unreachable (asleep / un-armed / off-network), **When** the operator uses the client, **Then** hub-side capabilities (browse shared memory, recall) function normally and host-side capabilities are shown as unavailable with the reason, never silently broken (R5; SC-205).
6. **Given** an attached remote session, **When** the operator deliberately closes it, **Then** the tunnel tears down and re-attaching requires re-arming on the desk machine; **but when** the session suffers a transient network drop while still armed, **then** the client re-attaches without re-arming (D5; SC-207).
7. **Given** an attached remote session, **When** the operator views the run, **Then** run-state events fold over the P2P channel without routing the live run through the hub — the hub's only role in the session is discovery / signaling / relay + identity, never execution or state relay of run-bearing content (Article VI; F-1; SC-206).

> **CL-203 (resolved):** fleet and presence are **deferred** — not in this slice; no acceptance scenarios written for them. US2's hub-side capabilities for this slice are **browse shared memory + recall** (FR-211).

---

### User Story 3 — Act on my machine remotely, every action gated and logged (Priority: P3 — in slice one per CL-203)

From the attached remote session (US2), the operator does the same things they do locally — not just watches. They answer a permission prompt, steer the run, and kick off a skill or agent as a run step (**run-control ①**). They run a non-interactive command (`npm test`, `git diff`, `git status`), read a file, and browse the file tree, with output piped back as streamed stdout/stderr (**dev-context ②**) — no terminal emulation. Every one of these actions routes through the **same permission + guardrail gate** the local run uses and lands on the **same event log**, so the remote action is run-aware, guardrailed, and auditable — the differentiator over a raw SSH session. Wagner does **not** build a raw interactive shell; for the rare true-interactive need the operator uses `ssh`/`tmux` over the tunnel that is already established.

**Why this priority:** Remote *agency* — "do what I do locally, from my phone" — is the headline differentiator over Claude Code's observe-only remote, but it depends on US2 (an authenticated, attached session) and on the carried gate + event log being the single chokepoint. It is sequenced last because run-control and dev-context are the highest-trust capabilities (they execute commands and read files on the operator's machine) and must inherit, not bypass, the existing guardrails. It is separable: US2 ships an observe + org-memory client; US3 adds the act layer on top of the same channel.

**Independent Test:** From an attached remote session, the operator (a) answers a pending permission prompt and the run advances; (b) runs `git diff` and a non-interactive `npm test` and sees streamed output; (c) reads a file and lists the file tree (within the configured file-read scope, CL-202); (d) launches a skill as a run step. Verify each action is reflected in the host's append-only event log attributed to the verified remote operator, each passed through the permission + guardrail gate, and no raw interactive PTY/shell was used — confirming run-control, dev-context, the single-gate chokepoint, and the audit trail.

**Acceptance Scenarios:**

1. **Given** an attached remote session with a run awaiting a permission decision, **When** the remote operator answers it, **Then** the same control message the local surface would send is delivered to the host, the run advances, and the decision is recorded on the event log attributed to the verified remote operator (D4, D6).
2. **Given** an attached remote session, **When** the remote operator runs a non-interactive command (`npm test`, `git diff`, `git status`), **Then** stdout/stderr stream back to the client as piped output (no PTY), and the command + its invocation are recorded on the event log (D4; FR-302).
3. **Given** an attached remote session, **When** the remote operator reads a file or lists the file tree, **Then** the read is served only within the configured file-read scope (CL-202) and is recorded on the event log; a read outside the scope is refused (FR-303; EC-006).
4. **Given** an attached remote session, **When** the remote operator launches a skill/agent as a run step, **Then** it executes on the host as a local run step would and is folded into the run's event log (FR-301).
5. **Given** any remote host-side action (run-control or dev-context), **When** it is requested, **Then** it passes through the **same** permission + guardrail gate as a local action — a remote action can never bypass a guardrail that would stop the local equivalent (D6; FR-304; SC-202).
6. **Given** an operator who needs a true interactive shell, **When** they reach for one, **Then** Wagner provides none of its own and the operator uses `ssh`/`tmux` over the established tunnel — raw interactive PTY is explicitly out of scope (D4; §Out of Scope).
7. **Given** remote dev-context streams file contents and diffs to the client, **When** that data moves, **Then** it travels edge→the operator's own authenticated device over the encrypted P2P channel and **never** to the hub — preserving the Article IX hub privacy boundary while enabling remote agency (F-1; SC-206).

---

### Edge Cases

- **EC-001 (network / NAT):** Both host and remote client are behind hard (e.g. symmetric) NAT and no direct P2P path forms. Expected: the channel falls back to a relay path and still attaches; if the relay is also unreachable, the remote attach **fails with a clear reason**, and the local run is wholly unaffected (continues, completes) — remote is additive, never a dependency of execution (Article VI; US2-AS-2).
- **EC-002 (lifecycle / host loss mid-session):** The operator closes the laptop lid or the host process dies while a remote session is attached. Expected: the remote client detects host-unreachable, drops host-side capabilities to "unavailable (host not reachable)", and **retains hub-side capabilities** (browse, recall); on the host returning and the session still armed, host-side capability re-attaches (R5; US2-AS-5/6).
- **EC-003 (security / auth):** A remote client presents an expired or invalid org SSO token. Expected: every remote action is refused before any channel opens; the host and the local run are unaffected; the operator must re-authenticate (D5; US2-AS-3).
- **EC-004 (security / arming):** A device attempts to attach to a host that was never armed for a remote session. Expected: refused — arming is an edge-machine-only action and a remote client can never bootstrap its own access (D5; SC-203; US2-AS-4).
- **EC-005 (concurrency / multiple clients):** Two remote clients (e.g. the operator's phone and a laptop browser) attach to the same host. Expected: both observe the same folded run state; control (steering, permission answers) is **first-write-wins through the single gate** (CL-204) — the first decision to reach the gate applies and a later conflicting decision for the same prompt is a no-op, so two conflicting answers can never both apply. There is no control-token hand-off; any attached client may send control, the gate serializes.
- **EC-006 (privacy / file-read scope):** Remote dev-context requests a file outside the run's repo, or a sensitive file (e.g. `.env`, an SSH key). Expected: file reads are **repo-scoped, default-deny** (CL-204→CL-202): reads are served only under the run's repo root; **any** path outside the repo root (including `~/.ssh`, `~/.aws`, `.env` files anywhere outside the repo) is refused and recorded as a **refused** action on the event log. Secrets outside the repo are unreachable by construction; an in-repo secret file is reachable (the operator owns the repo) but the read is still gated + logged.
- **EC-007 (window vs quit):** The operator closes the window (host stays alive, EC ↔ US1-AS-1) versus fully quits the app (host + endpoint stop). Expected: window-close backgrounds to tray and keeps the host running; full quit stops the host, drops any attached remote session to hub-side-only, and requires re-launch + re-arm to reconnect — the intended, explicit boundary (R4; US1-AS-5).
- **EC-008 (transport / discovery loss):** The hub (which provides discovery / signaling / relay) is unreachable when a remote client tries to attach. Expected: a **new** attach cannot complete (discovery needs the hub), and hub-side capabilities (browse, recall) also return empty/unavailable; but an **already-established** direct P2P session continues over its existing path, and the local run is unaffected (Article VI; D-RES-2; EC-001).
- **EC-010 (boundary / re-arm while armed):** The operator calls `arm` on a host that is **already armed** with an active discovery registration. Expected: the re-arm is **idempotent-refresh** — it re-registers the same host under the operator's identity, **refreshes** the NodeId/signaling ticket and its expiry, and emits a single new `remote.armed` event (the prior registration is replaced, not duplicated); no second concurrent session or duplicate registry row is created (CL-204 first-write semantics extended to arming). A re-arm never originates from a remote client (FR-201 — edge-only).
- **EC-011 (boundary / transient-drop expiry at the threshold):** A transient drop lasts **exactly** the re-attach window. Expected: the re-attach-without-re-arm window is **exclusive at the upper bound** — a drop of strictly **< 30 s** re-attaches without re-arming; a drop of **≥ 30 s** has let the discovery registration lapse and **requires re-arming** (the server-side registration TTL uses `< 30 s`). This fixes the exact-boundary behaviour so SC-007 and its test (T025) are unambiguous (challenge M2).
- **EC-009 (a11y on mobile / tray):** Run state on the constrained mobile surface and the tray glyph. Expected: state carries a non-color glyph + text label on every surface; every animated liveness cue (live activity strip) has a `prefers-reduced-motion` alternative; body text clears WCAG 2.1 AA contrast on the dark palette (D-A11Y-1; `apps/wagner/PRODUCT.md` §Accessibility).

---

## Functional Requirements *(mandatory)*

**Foundational — the projection model, identity, and the gate (underlie US1, US2, US3)**

- **FR-001:** The operator-facing surface MUST be a **single codebase** that renders in three environments — a desktop (Tauri) shell, a web browser, and mobile (responsive) — and MUST derive run state by folding the host's append-only event stream through the **same pure reducer** in every environment (R1; Article VIII; D1). No environment may carry its own divergent run-state logic.
- **FR-002:** The surface MUST obtain the event stream over a **transport-abstracted** channel so that a local surface (desktop, in-process/IPC channel) and a remote surface (web/mobile, P2P channel) fold the *same* stream with no surface-level difference in run rendering (D1; R1). The transport is a detail below the reducer.
- **FR-003:** Capability availability MUST be a function of **host-reachability** (R5): a surface MUST always offer hub-side capabilities (browse shared memory, recall, and — pending CL-203 — fleet, presence) which require no host; host-side capabilities (run-control, dev-context) MUST be offered only when a host is reachable, and MUST be shown as unavailable-with-reason (never silently broken) when not.
- **FR-004:** Every **remote** host-side action MUST route through the **same permission + guardrail gate** as the equivalent local action, and MUST be appended to the run's event log; there MUST be no remote code path that reaches the run engine while bypassing the gate (D6; carried 001 gate + event spine). This single chokepoint is the audit trail and the guardrail guarantee.
- **FR-005:** Every message crossing the remote channel (control, dev-context request/response, attach/arm handshake) MUST validate against a declared JSON Schema (draft 2020-12, `additionalProperties: false`) before it is acted on (Article X; carried boundary discipline).

**Remote identity, arming, and the channel (underlie US2, US3)**

- **FR-200:** A remote operator MUST be verified by **org SSO (OIDC against Google / JumpCloud, ADR-0002)** before any remote channel opens; the remote operator identity attributed to a remote action is the IdP-issued subject, reusing the wedge-001 identity mechanism (D-IDENT-1; FR-200 reuses ADR-0002, no new auth mechanism).
- **FR-201:** A host MUST be reachable to remote clients only after an explicit **arm** action performed **on the edge machine**; a remote client MUST NOT be able to arm a host or bootstrap its own access (D5). Arming is the operator's deliberate, edge-side opt-in to remote agency.
- **FR-202:** A remote session MUST be **ephemeral**: a deliberate close MUST tear down the tunnel such that re-attachment requires re-arming on the edge machine; a transient network drop **while the session is still armed** MUST allow re-attachment without re-arming (D5).
- **FR-203:** The remote transport MUST be an **authenticated, encrypted, NAT-traversing P2P channel** between the operator's edge host and the operator's remote client, with **relay fallback** when a direct path cannot form. The relay MUST be an **org-run relay** (CL-201) — connection metadata stays in-house; the relay carries only opaque encrypted transport frames (F-1). The **hub** provides only discovery / signaling + relay coordination and identity — it MUST NOT execute run work or carry run-state-bearing content as plaintext (D3; Article VI; F-1). *(Org-run relay is an Article V Complexity-Tracking item — see plan; transport = iroh per design.md D3, recommended ADR-0003.)*
- **FR-204:** When the host is unreachable from a remote client (un-armed, asleep, off-network, or hub discovery down), hub-side capabilities MUST degrade gracefully (browse/recall return their best available result or empty, never an error that blocks the client) and host-side capabilities MUST be presented as unavailable with the reason (R5; D-RES-2; Article VI).

**User Story 1 — tray-resident host + unified local surface**

- **FR-100:** The execution **host MUST be a native process** (never a browser); web and mobile surfaces are necessarily remote clients of a native host (R3). The host embeds in the Tauri desktop app for this slice (R4; D-PROJ-4).
- **FR-101:** Closing the desktop **window MUST background** the app to the menu bar / system tray rather than exiting; the host process and its append-only event log MUST keep running while the window is closed (R4). **When the host is armed (US2), the remote endpoint MUST also survive window-close** so remote clients keep attaching — this endpoint-survival clause is verified once the endpoint exists, in US2 (the iroh endpoint is US2 scope), not in the US1 host-lifecycle test which has no endpoint to assert (resolves the FR-101↔phase-ordering gap, challenge C2).
- **FR-102:** The tray MUST display the current run state — at minimum **idle / running / needs-you** — using a **non-color glyph + text label** (D-A11Y-1), and MUST update as the run state changes even while the window is closed (R6).
- **FR-103:** When a run reaches a **needs-you** state (a permission prompt or a tripped guardrail) while the window is closed, the system MUST raise a native OS notification and a tray badge within the liveness threshold (SC-008; `apps/wagner/PRODUCT.md` principle 3/4).
- **FR-104:** **Quitting the app** (distinct from closing the window) MUST stop the host process and its remote endpoint; this boundary MUST be explicit to the operator (R4; EC-007).
- **FR-105:** Reopening the window MUST render the live run state folded from the host's event log with no loss or divergence from what the host holds (Article VIII; US1-AS-2).

**User Story 2 — remote observe + hub-side capabilities**

- **FR-210:** Given an armed host and a verified remote operator, a remote client MUST **discover and attach** to the host over the P2P channel (FR-203) and fold its event stream to render the live run identically to the local surface (D1; US2-AS-1).
- **FR-211:** Hub-side capabilities — **browse shared memory** and **recall** (the wedge-001 hub data) — MUST function on a remote client with **no host reachable** (R5). *(CL-203: fleet and presence are deferred — not in this slice.)*
- **FR-212:** A remote attach MUST be **refused** when the host was not armed (FR-201) or the operator is not verified (FR-200); the local run MUST be unaffected by a refused attach (US2-AS-3/4; EC-003/004).

**User Story 3 — remote act (host-side capabilities)**

- **FR-301:** **Run-control ①** — a remote operator MUST be able to start/steer a run, answer a permission prompt, and launch a skill/agent as a run step — using the **same control messages** as the local surface, delivered over the P2P channel and gated + logged per FR-004 (D4; US3-AS-1/4).
- **FR-302:** **Dev-context ② (commands)** — a remote operator MUST be able to run **non-interactive** commands (e.g. `npm test`, `git diff`, `git status`) on the host with stdout/stderr **piped** back as streamed output (no PTY); the invocation MUST be gated + logged (D4; US3-AS-2). Raw interactive shell is out of scope (use `ssh`/`tmux` over the tunnel).
- **FR-303:** **Dev-context ② (files)** — a remote operator MUST be able to **read a file** and **list the file tree** on the host, served only **within the run's repo root (repo-scoped, default-deny — CL-202)**, with the read gated + logged; a read resolving to any path outside the repo root MUST be refused and logged as refused (US3-AS-3; EC-006). The scope check MUST resolve symlinks/`..` traversal before the allow decision (no escape via path tricks).
- **FR-304:** No host-side remote action (FR-301–303) may **bypass a guardrail** that would stop the local equivalent; the gate decision MUST be identical regardless of whether the action originated locally or remotely (D6; FR-004; SC-202).
- **FR-305:** Remote dev-context payloads (file contents, diffs, command output) MUST travel **edge → the operator's own verified device** over the encrypted P2P channel and MUST NOT be routed to or stored on the hub — the hub privacy boundary (Article IX) is preserved because nothing additional reaches the hub (F-1; SC-206).

---

## Success Criteria *(mandatory)*

- **SC-001:** A verified remote operator on a **different network behind NAT** attaches to an armed host and sees the first folded run-state frame within **p50 ≤ 3 s of initiating attach** (target provisional; measured over a residential-NAT-to-residential-NAT path with relay fallback available — conditions fixed so the criterion is testable). (US2 reach.)
- **SC-002:** **100%** of remote host-side actions (run-control + dev-context) appear in the host's append-only event log attributed to the correct verified remote operator, and **0** remote actions reach the run engine without a gate decision. (D6 audit + FR-004 single chokepoint — a hard zero.)
- **SC-003:** **0** remote attachments succeed against a host that was not armed on the edge machine, across all attempts. (D5 no-self-arm — a hard zero.)
- **SC-004:** **0** in-flight runs are interrupted by a window-close; a run executing when the window closes completes at the same rate as one with the window open. (US1 tray-resident host.)
- **SC-005:** Hub-side capabilities (browse, recall) function on a remote client with the host unreachable **100%** of the time (subject to the hub itself being reachable); host-side capabilities are shown unavailable-with-reason, never as a silent failure. (R5 graceful degradation.)
- **SC-006:** **0 bytes** of code, file content, diff, or transcript are **stored on, or readable in plaintext by, any hub application service** (discovery, sync, recall, store) as a result of any remote session. The org-run relay, when used, forwards only **opaque encrypted transport frames it cannot decrypt** — those ciphertext frames are explicitly **not** a privacy violation (F-1) and are excluded from this count. Verified by: (a) hub-side storage gains no code/diff/file/transcript field after a remote dev-context session (the metadata-only schema enforces this structurally), and (b) the relay records frame **sizes** only, never a decrypted payload. (Article IX preserved; F-1 — a hard zero on hub-readable run-bearing content.)
- **SC-007:** A **deliberately closed** remote session cannot be re-attached without re-arming on the edge machine (0 re-attach-without-re-arm successes); a **transient drop of strictly < 30 s** while still armed re-attaches without re-arming ≥ 95% of the time, and a drop of **≥ 30 s** requires re-arming (exclusive upper bound — EC-011). (D5 ephemerality — targets provisional, the boundary is exact.)
- **SC-008:** A `needs-you` notification (permission prompt / tripped guardrail) is raised to the operator within **≤ 5 s** of the run entering that state while the window is closed (target provisional; the liveness contract from `apps/wagner/PRODUCT.md` principle 4, made measurable per D-PERF-1). (US1 needs-you surface.)

> SC targets marked provisional are confirmed in plan-phase perf work (Article II — recorded as Assumptions, not silent defaults). The **measurement conditions** are fixed here so each criterion is testable now.

---

## Key Entities *(include only when feature involves data)*

- **Surface (client):** The operator-facing GUI. One codebase; three render environments (desktop-Tauri / web / mobile-responsive). Stateless w.r.t. run truth — it folds the host event stream via the carried pure reducer (Article VIII). Attribute: `transport` (in-process | p2p). New framing introduced by this feature (the carried `apps/wagner` UI is desktop-only).
- **Host:** The native execution process. Owns the goal loop, the permission + guardrail gate, the append-only event log (carried 001 spine), and the remote endpoint. Lifecycle: `embedded-in-Tauri` for this slice; backgrounds to tray on window-close; stops on app-quit (R3, R4). Headless daemon packaging is a later step over the same host crate (out of scope here).
- **Remote session:** An authenticated, edge-armed, ephemeral attachment from a remote client to a host over the P2P channel. Lifecycle: `unarmed → armed (edge action) → attached (remote, SSO-verified) → (detached-transient → re-attached | closed → torn-down, must re-arm)`. New entity (D5).
- **Capability channel:** A scoped channel over the session exposing **one** capability tier — run-control ① or dev-context ② — not a whole-machine tunnel ("app-defined authenticated streams, not a VPN", design.md D3). Each message schema-validated (FR-005). New entity.
- **Tray presence:** The menu-bar/system-tray projection of run status — `idle | running | needs-you` — with a non-color glyph + label, a badge, and native notifications (R6, D-A11Y-1). New entity (a surface, not run truth).
- **Operator identity (reused):** The verified person (employee) a remote action is attributed to; established by OIDC SSO (Google/JumpCloud), `operator_id` = the IdP subject (ADR-0002; wedge-001). Unchanged by this feature — reused at the remote-channel boundary.
- **Run / Event (reused):** The carried run aggregate and its append-only event log (wedge-001). This feature adds **new event kinds** for remote/host-lifecycle actions (arm, attach, remote-control, dev-context-command, dev-context-read, detach) so the audit trail is complete; it does not change the run aggregate's meaning.
- **Hub (reused, role extended):** The wedge-001 Deno + Hono + SurrealDB hub (ADR-0001). For this feature its role **extends to discovery / signaling + relay** for the P2P channel, on top of identity and the 001 memory/recall it already serves. It MUST NOT execute run work or carry run-state-bearing content (Article VI; F-1).

---

## Assumptions

- The run engine, permission + guardrail gate, append-only event log, and pure reducer are **carried from wedge 001 / `apps/wagner`** and are not reinvented; this feature adds surfaces and a remote path over the existing spine (design.md §"Chosen approach"; D-PROJ-4).
- The edge stack stays **Rust host + TypeScript frontend (Tauri)** (D-PROJ-4; R1/R3/R4). The unified surface is the same TS frontend rendered in three environments, not three codebases (R1).
- Identity is the **wedge-001 OIDC SSO** mechanism (ADR-0002); this feature introduces **no new auth mechanism**, only applies it at the remote-channel boundary (FR-200).
- The remote operator and the edge operator are the **same trusted employee**; a hostile-operator / multi-tenant threat model is out of scope (carried from wedge-001's single-trusted-org assumption).
- The run stays on the operator's machine (**model (a)** — a remote client reaches *in*); cloud/hybrid execution is deferred (design.md D3 trade-offs).
- The remote transport is **iroh** (QUIC P2P, Ed25519 node identity, NAT traversal) with an **org-run relay** (design.md D3; CL-201). The org-run relay is a new operational surface — an Article V Complexity-Tracking item recorded in plan.md (recommended ADR-0003).
- Concurrent remote control is **first-write-wins through the single gate** (CL-204) — no control-token hand-off; the gate serializes and a later conflicting decision for the same prompt is a no-op (EC-005).
- **Provisional values to confirm in plan-phase** (recorded as Assumptions per Article II, not silent defaults): remote attach p50 ≤ 3 s and its NAT measurement conditions (SC-001); transient-drop window ≤ 30 s for re-attach-without-re-arm (SC-007); needs-you notification ≤ 5 s (SC-008).

### Defaults Applied

- `D-IDENT-1` — FR-200 (operator identity attributed at the remote-channel boundary).
- `D-A11Y-1` — FR-102, US1-AS-4, EC-009 (non-color glyph + label on tray/mobile; reduced-motion; AA contrast).
- `D-SEC-1` / `D-SEC-3` — host execution still drives subscription CLIs with no metered API key (carried); every channel message schema-validated (FR-005).
- `D-RES-2` — FR-204, EC-001, EC-008 (graceful degradation when host/hub unreachable; run never blocked).
- `D-OBS-1` / Article X — FR-005 (every channel message is a schema-validated event on the stream).
- `D-STORE-2` / Article VIII — FR-001, FR-105 (surfaces fold the append-only log via the pure reducer).
- `D-PROJ-2` — remote agency is modeled as **new event kinds over the existing run/event spine**, not a new subsystem (constitution; `platform/prd.md` §"the run primitive").
- `D-PROJ-3` — any new operator-facing copy uses platform vocabulary (agent / engine class / stage / planner), no floor terms.
- `D-PROJ-4` — Assumptions (edge stack = Rust host + TS frontend, Tauri).

### Defaults Overridden

- *(none — this feature reuses wedge-001's overridden defaults: identity = OIDC SSO (D-IDENT-2 → ADR-0002), hub = SurrealDB/Deno (D-STORE-5/D-PROJ-6 → ADR-0001). No new overrides.)*

---

## Out of Scope

- **Raw interactive shell / PTY / browser terminal** (tier ③) — Wagner builds none; the rare true-interactive need is served by `ssh`/`tmux` over the already-established P2P tunnel (D4). This is the deliberate avoidance of cross-platform PTY + xterm.js.
- **Headless daemon packaging** (the "host runs with no app open at all" endgame) — a later packaging step over the same Tauri-independent host crate; this slice is **embedded-in-Tauri, backgrounds-to-tray** only (R4).
- **A native TUI surface** — demoted to optional/later (R2); the engine is surface-agnostic so a TUI may be added later, not now. (`tui-design` is the counterpart skill if/when it is built.)
- **Retiring the carried Tauri GUI** — the GUI becomes the cross-environment **primary**, not a retired phase (R1/R2).
- **Cloud / hybrid run execution** — the run stays on the edge (model (a)); hub-side or cloud execution of edge runs is forbidden by Article VI and deferred as a Phase-2+ strategic alternative.
- **The hub executing run work or relaying run-state-bearing content** — the hub's remote role is strictly discovery / signaling / relay + identity (Article VI; F-1).
- **A hostile-operator / multi-tenant isolation threat model** — single trusted org (carried from wedge-001).
- **Windows / Linux desktop packaging guarantees** — macOS desktop is primary for this slice (carried target from wedge-001 plan); cross-platform *reach* is delivered by the web/mobile remote client (R1), **not** by a guaranteed native desktop build on every OS. *(CL-205 resolved by default: R1's "web/mobile reach forces the web GUI" means the web client satisfies cross-OS reach this slice; a native Windows/Linux desktop build is deferred. Engineer may override in a later amend.)*
- **Automatic learning extraction, semantic recall, sensors, Temporal/workers** — all remain wedge-001 / Phase-2+ out-of-scope; unchanged here.

---

## Dependencies

- **The carried run spine (wedge 001 / `apps/wagner`):** the goal loop, permission + guardrail gate, append-only event log, and pure reducer that every surface folds and every remote action routes through. Failure mode: N/A (in-repo source); this feature is additive over it.
- **iroh P2P transport + an org-run relay (NEW — Article V Complexity-Tracking item; transport = iroh per design.md D3, relay = org-run per CL-201; recommended ADR-0003):** authenticated (Ed25519 node identity), encrypted, NAT-traversing channel edge↔client; the org-run relay is a fallback path when no direct connection forms and a new deploy/operate surface. Failure mode: no direct path + relay unreachable → remote attach fails gracefully (EC-001); local run unaffected (Article VI).
- **The wedge-001 hub (Deno + Hono + SurrealDB, ADR-0001), role extended to discovery / signaling + relay:** lets remote clients find and reach an armed host, and serves the hub-side browse/recall. Failure mode: unreachable → no **new** remote attach and hub-side capabilities empty/unavailable, but an already-established P2P session and the local run continue (EC-008; Article VI).
- **The org IdP (Google / JumpCloud via OIDC, ADR-0002):** verifies the remote operator before any channel opens. Failure mode: IdP unreachable / token expired → no remote session opens; the local run starts and completes regardless (Article VI; EC-003).
- **Tauri tray / window-lifecycle APIs (carried):** menu-bar presence, close-to-tray, native notifications, badge. Failure mode: N/A (in-repo dependency); platform-specific behaviour (macOS `ActivationPolicy::Accessory`) noted in plan.
- **Subscription `claude`/`codex` CLIs (carried):** host-side execution for run-control and dev-context. Failure mode: absent/unauthenticated → a run cannot start (carried preflight), independent of the surface.

---

## Constitution Addenda *(optional)*

- **F-1 (Remote channel vs the hub privacy boundary):** A remote session is an **edge-to-operator's-own-device** channel over an authenticated, encrypted P2P link. Run execution and run-state-bearing content (code, diffs, file contents, command output, transcripts) MUST flow only over that edge↔device channel and MUST NOT be **stored on, or readable in plaintext by, any hub application service** (discovery, sync, recall, store). The **org-run relay is a distinct, zero-knowledge transport forwarder**: it MAY carry the E2E-encrypted QUIC frames of a remote session (that is its job — the NAT fallback path), but it forwards **opaque ciphertext it cannot decrypt** and logs frame **sizes** only, never decrypted content. Encrypted relay frames are therefore **not** a privacy violation. The hub's application role for a remote session is strictly **discovery / signaling + identity**; the relay's role is strictly **opaque frame forwarding**. This keeps Article VI (edge executes, hub remembers) and Article IX (only metadata + curated learnings reach the hub) intact while enabling remote agency. **How to verify:** a test asserts (a) hub storage gains **no** code/diff/file/transcript field after a remote dev-context session (structurally enforced by the metadata-only schema, `additionalProperties:false`), and (b) the relay records frame sizes only — never a decrypted payload (SC-006). Inspecting raw relay traffic is **not** the verification method (it would observe ciphertext frames by design).

---

## Cross-Plugin Surfaces *(optional)*

Not a multi-plugin feature. This feature spans the platform runtime's edge layer (`platform/edge/host`, `platform/edge/ui`), the shared spine (`platform/shared`), and extends the hub (`platform/hub`) with discovery/signaling/relay. It touches the capability library only as a one-directional consumer (Article VII). No `plugins/*` changes.

---

## Clarifications

Filled in by `/spec clarify`. Open markers carried into Phase 2:

- **CL-201** (transport / relay) — iroh relay operator-/org-run vs public/third-party (cost / latency / privacy). Impact: scope + security. (FR-203.)
- **CL-202** (privacy / dev-context) — tier-② file-read scope: any-path vs repo-scoped, and the protected-path policy. Impact: security/privacy. (FR-303, EC-006.)
- **CL-203** (scope / slice one) — which capabilities land in the first shippable slice; specifically whether **fleet** and **presence** are in US2's first remote slice or deferred. Impact: scope. (US2, FR-211.)
- **CL-204** (concurrency) — concurrent remote control: single-active-controller vs first-write-wins through the gate. Impact: UX/technical. (EC-005.)
- **CL-205** (scope / platform) — native Windows/Linux desktop build in this slice, or cross-OS reach via the web client only. Impact: scope. (§Out of Scope.)

### Session 2026-06-15

- **CL-203** (scope / slice one) → **Full agency.** Slice one = US1 + US2 + **full US3** (run-control ① **and** dev-context ②). **Fleet** and **presence** deferred to a follow-up slice. US1 remains the independently-shippable P1 MVP; the slice-one definition of done includes all three stories (mirrors wedge-001 CL-1). Applied to the §User Scenarios slice-one banner, US2 (FR-211, removed the fleet/presence marker), US3 header.
- **CL-201** (transport / relay) → **Org-run relay.** Payloads are E2E-encrypted (F-1) so the relay only carries ciphertext, but connection metadata stays in-house. Recorded as an Article V Complexity-Tracking item (plan.md) + recommended ADR-0003. Applied to FR-203, §Dependencies, §Assumptions.
- **CL-202** (privacy / dev-context) → **Repo-scoped, default-deny** file reads: reads served only under the run's repo root; any path outside (incl. `~/.ssh`, `~/.aws`, out-of-repo `.env`) refused + logged; symlink/`..` traversal resolved before the allow decision. Applied to FR-303, EC-006.
- **CL-204** (concurrency) → **First-write-wins through the single gate.** No control-token hand-off; the gate serializes; a later conflicting decision for the same prompt is a no-op. Applied to EC-005, §Assumptions.
- **CL-205** (scope / platform) → **Resolved by default**: cross-OS reach is satisfied by the web/mobile client (R1); a native Windows/Linux desktop build is deferred; macOS desktop stays primary this slice. Applied to §Out of Scope. Engineer may override in a later amend.
