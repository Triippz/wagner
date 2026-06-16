# ADR-0003: Remote transport via iroh (QUIC P2P) with an org-run relay

**Date:** 2026-06-16
**Status:** Accepted

## Context

Wedge `platform/specs/002-edge-surface-and-remote-sessions` adds remote sessions with
agency: an operator on a second device reaches a running edge host to observe and act
(answer permissions, steer, run skills, run non-interactive dev-context commands, read
repo-scoped files). The run stays on the edge; the remote client reaches *in*. Four facts
constrain the transport:

1. **The run must never depend on the hub or the remote path (Article VI).** A remote
   action is a message into the *already-local* gate; the hub does discovery/signaling
   only and never executes. Any transport that routes run-bearing content through the hub
   fails Article VI on its face. Offline-completion is asserted by T009a.
2. **No run-bearing content may cross the edge→hub boundary, even during a remote session
   (Article IX; F-1).** Code, diffs, file contents, and transcripts flow edge→operator
   device only. The transport must therefore be end-to-end encrypted so that any relay on
   the path is zero-knowledge of content.
3. **Peers must be authenticated as both a device and a person.** A channel opens only
   after Ed25519 node identity *and* OIDC operator identity (ADR-0002) both verify
   (FR-200, FR-203). The remote client may attach only to a host it owns, and only if the
   host is **armed** (FR-201 — arming is edge-only; self-arm must be structurally
   impossible).
4. **The peers are residential-NAT↔residential-NAT.** A direct path will not always form;
   a fallback is required (EC-001), and the engineer chose to keep connection metadata
   in-house rather than transit a third party (CL-201).

The transport mechanism was left to the plan phase (design.md D3–D6). This is a new,
load-bearing dependency — it deserves a recorded decision, not just plan prose.

## Decision

Remote transport is **iroh** (QUIC, Ed25519 node identity, built-in NAT traversal) with an
**org-run iroh relay** as the discovery-assisted hole-punching aid and symmetric-NAT
fallback. The decision has four parts:

- **Transport = iroh `Endpoint`, one app-defined ALPN per capability channel** (R-1; D3).
  QUIC gives end-to-end encryption; Ed25519 node identity authenticates the peer device;
  per-ALPN streams map cleanly onto the capability tiers — `wagner/attach/1`,
  `wagner/control/1`, `wagner/devctx/1`. E2E encryption is what lets the relay stay
  zero-knowledge (F-1). No PTY/interactive-shell ALPN is registered (US3-AS-6; tier ③ is
  ssh/tmux over a tunnel, not built into Wagner — T037b enforces this).

- **Relay = org-operated, zero-knowledge** (R-2; CL-201). The org stands up one iroh relay;
  edge host and remote client use it for hole-punching and as a fallback path when no
  direct connection forms. It forwards only opaque encrypted QUIC frames it cannot decrypt
  and logs frame **sizes** only (SC-006). Carrying ciphertext is explicitly **not** a
  privacy violation (F-1; challenge H1). The relay is the one new operational process; the
  org already runs the IdP and the hub.

- **Discovery/signaling folds into the existing hub** (R-3). `POST /discovery/register`
  (OIDC-gated; stores an ephemeral `{operator_id, node_id, ticket, expires_at}`) and
  `POST /discovery/resolve` (owner-only; 404 if not armed/not owned). No standalone
  discovery service, no hub-stored long-lived tokens. The hub touches signaling only,
  never run execution.

- **Arming + ephemeral session lifecycle** (R-3; D5). `arm` is an **edge-only** host action
  (advertises the endpoint, registers NodeId/ticket, emits `remote.armed`) — there is no
  host API by which a remote peer can arm, so self-arm is structurally impossible
  (FR-201, SC-203). Sessions are ephemeral: deliberate close tears down and requires
  re-arm; a transient drop **< 30 s** while armed silently re-attaches; a drop of
  **≥ 30 s** requires re-arm (SC-007, exclusive boundary). Arm/attach/detach are
  new event kinds folded by the carried reducer (Gate VIII), so the session is a
  replayable, attributable projection like everything else.

OIDC operator identity is reused unchanged from ADR-0002; this ADR adds the device-identity
and transport layer beneath it. Remote actions route through the **single carried gate**
(`permission_server.rs` + `guardrails.rs`) — no second gate (R-7; FR-004/304).

## Alternatives Considered

- **WebSocket through the hub.** Rejected: forces run-state-bearing content through the hub
  — fails Article VI (hub on the run path), Article IX, and F-1 (hub would see plaintext).
- **WebRTC data channels.** Rejected: heavier signaling, browser-centric, and a weaker
  server-side identity story than iroh's Ed25519 node identity.
- **SSH tunnel as the primary transport.** Rejected: that is dev-context tier ③, the thing
  we are explicitly *not* rebuilding (D4); operators use ssh/tmux over a tunnel for an
  interactive shell, and Wagner registers no PTY ALPN.
- **iroh public relays.** Rejected per CL-201: connection metadata would transit a third
  party. Payloads are E2E-encrypted either way, but the engineer chose to keep metadata
  in-house.
- **Direct-only / no relay.** Rejected: no fallback behind symmetric NAT (EC-001 would have
  no path).
- **Always-listening host (no arm gate).** Rejected: violates D5 opt-in and enlarges the
  attack surface; arming is the consent + audit point.
- **Hub-stored long-lived attach tokens.** Rejected: makes the hub a credential authority —
  the same surface ADR-0002 and the 001 R-1 rejection avoid.

## Consequences

- **Easier:** the run critical path stays hub-free and remote-free (Article VI holds by
  construction — §1.4 proof, T009a); the relay is zero-knowledge so F-1/Article IX hold even
  mid-session; per-ALPN channels give a clean capability-tier seam; reusing the carried gate
  guarantees no remote action bypasses a guardrail (SC-202) and the audit trail is complete
  (SC-002); routing arm/attach/detach through the event log gives the session a free,
  replayable audit trail.
- **Harder / accepted:** the edge host gains an iroh dependency (T001) and a `remote/`
  module (endpoint, arm, session, control, devcontext); the org operates **one new process**
  (the relay) — a real Gate V complexity entry, accepted because remote *agency* is the
  feature and direct-only has no NAT fallback; the hub gains a discovery/signaling route +
  an ephemeral registry table. Remote-attach latency is a new perf surface, bounded by
  SC-001 (first-frame p50 ≤ 3 s over residential-NAT↔residential-NAT with relay available;
  T044/T045).
- **Bounded:** no message broker, no queue, no Temporal, no second datastore. The only new
  operational surface is the single relay process.

## Relationship to the spec tree

Records (does not supersede) plan §Phase 0 R-1, R-2, R-3, and the four Gate V Complexity
Tracking rows (iroh, org-run relay, hub discovery/signaling registry, Tauri tray/window
wiring), plus design.md D3–D6 and clarification CL-201. Builds on ADR-0002 (OIDC operator
identity) and ADR-0001 (Deno/SurrealDB hub the discovery route extends). Authored per task
T043; ALPN strings and channel schemas are authored as their own tasks (T010, §1.2).
