# ADR-0004: Deployment topology — sovereign headless engine, thin attach clients, personal↔enterprise continuum

**Date:** 2026-06-22
**Status:** Accepted (topology + voice consequence); enterprise extension records direction, with open questions deferred.

## Context

Wagner is shipping per-platform installers (DMG / MSI+NSIS / AppImage+deb+rpm). That
raised three questions that go deeper than packaging:

1. **It is not desktop-only.** The engine can run **headless anywhere** — a laptop, an
   always-on personal box, or org infrastructure — and a person ties in **remotely from
   any device** (a second desktop, a future mobile app, a browser tab).
2. **How does any of this work in a browser?** A browser tab cannot spawn the `claude`/
   `codex` CLI agents or the whisper/TTS sidecars, cannot open the mic via `cpal` (it has
   `getUserMedia`), and cannot touch the vault/project files.
3. **What does "enterprise" mean?** A team version where some things are offloaded to a
   server (the hub already persists shared knowledge), usable equally by one person or an
   org.

Most of the answer is already decided and only needs to be *named together*:

- **The engine is already headless.** `architecture.md`: *"host/ — Rust engine, headless,
  Tauri-free. The brain"*; the **hub** is *"a headless always-on peer"* that persists the
  shared vault and brokers.
- **Remote attach is decided** — **ADR-0003** (iroh QUIC P2P, org-run zero-knowledge
  relay): a client attaches to a host it **owns**, only when **armed**; the run stays on
  the edge; no run-bearing content crosses edge→hub (Article VI/IX).
- **Identity is decided** — **ADR-0002** (operator identity via SSO-OIDC).
- **Tenancy is a designed seam** — `runtime-architecture.md §7`: the Envelope carries
  `origin` (ParticipantId) + `scope` (user/workspace) from day one, so *"multi-tenant
  later is a subscription filter, not a migration… the future is additive: Cloud = add a
  peer, Teams = a scope filter + membership."*

## Decision

**1. The engine is a sovereign, headless, transport-agnostic artifact.** `host/` is the
brain and stays Tauri-free. It ships in two host shells over the *same* engine: (a)
embedded in the desktop app (Tauri), and (b) a standalone **headless daemon** as a
first-class build artifact. The engine exposes **one** capability surface — typed
commands + an event stream — over interchangeable transports:

| Transport | Used by |
|---|---|
| Tauri IPC (in-process) | the desktop app's own UI |
| localhost HTTP/WS | a same-machine browser tab |
| iroh (ADR-0003) | a remote client (other desktop, mobile, browser) attaching to an owned, armed engine |

**2. Clients are thin and heterogeneous; the engine is the tenant's control plane.**
Desktop GUI, browser tab, and a future mobile app are all **clients that attach** — none
runs the engine. One engine = one tenant's control plane; N clients per engine. The
engine's *host* may be a laptop, an always-on personal machine, or org infra; the client
topology is identical in every case.

**3. Voice follows the same rule — capture is per-client, intelligence is engine-side.**
The abstraction boundary is the **transcript**, not the mic:

| Surface | Capture | Transport | STT runs on |
|---|---|---|---|
| Desktop (Tauri) | `cpal` `MicCapture` | Tauri IPC | the edge (sidecar) |
| Local browser | `getUserMedia` → WAV in JS | localhost HTTP/WS | the edge (sidecar) |
| Remote client | `getUserMedia` (or mobile mic) | iroh attach | **the owner's edge** |

STT / intake routing / bus / projection are **engine-side and shared** across every
surface. **Transcription happens on the user's own edge — never the cloud** — which is
what keeps the constitution's privacy boundary intact (no raw audio/transcript to a third
party). `cpal` `capture.rs` is the *native arm*; a browser arm is additive (a JS capture +
an engine-side audio-ingest endpoint), reusing the same engine pipeline.

**4. Personal and enterprise are one architecture.** Enterprise is the personal model plus
layers, not a different system:

- **Identity** (ADR-0002) names *who* is attaching.
- **Tenancy** is the existing `scope` (owner/workspace) subscription filter (runtime-arch
  §7) — org membership selects which scopes a person sees.
- **A shared org tier in the hub** — curated knowledge graph, the agent/workflow/connector
  catalog, and team run-**metadata** dashboards. This is the "offload to a server": the
  hub holds *shared, curated* state and coordinates; it **never executes runs or holds
  run-bearing content** (Article VI/IX) for someone's private machine.
- **Sharing is configurable policy, not the hardcoded default.** The constitution's
  boundary (curated/metadata syncs; raw code/secrets/transcripts stay local) becomes the
  *personal default* of one policy mechanism the engine consults; an org sets its own tier
  + per-item gating on the same seam.
- **Org-hosted execution is the same engine daemon in org infra** — for team automation
  against *org-owned* repos (not a member's private machine). That is org execution, not
  offloading a user's local runs.

## Consequences

Build discipline to hold **now**, so every surface above stays additive:

1. **Keep engine logic out of the Tauri shell.** The headless daemon is a real artifact; if
   a capability only works through `edge/shell`, it won't work headless/remote/browser.
2. **The engine API is transport-agnostic.** The UI's `TauriBridge` is the client-side
   mirror — new transports (HTTP/WS, iroh) slot in behind the same `cmd` interface without
   touching call sites.
3. **The sync/privacy boundary is policy the engine consults**, not an `if` branch — so the
   personal default and an org policy share one enforcement point.
4. **Voice keeps capture + transport swappable, STT-onward engine-side** (already factored
   this way in `voice/capture.rs`).

Per runtime-arch §7, the future then stays additive: browser = a transport + a JS capture;
mobile = a thin client; teams = scope filter + hub tier + membership; org execution = run
the engine daemon in org infra.

## Open questions (deferred — not decided here)

- **Org-hosted execution engines** — isolation, secrets, and identity model for running the
  daemon against shared repos in org infra.
- **Sharing-policy model** — admin controls, per-tier and per-item gating, and the single
  enforcement point in the sync layer.
- **Personal-UX vs enterprise tension** — `ui-redesign-findings.md` rejected a v1 design as
  *"too enterprisey."* Enterprise must stay opt-in and never intrude on the solo experience.
- **Sequencing** — desktop-first (current) → headless daemon + local browser → remote
  attach (ADR-0003) → org tier. Each step ships independently.

## Relationship to the spec tree

Builds on **ADR-0002** (operator identity / SSO-OIDC), **ADR-0003** (remote transport via
iroh + org relay), `docs/runtime-architecture.md §7` (forward-compat seams: serializable
events, `origin`+`scope`, typed bus), and `docs/architecture.md` (headless host + hub). It
constrains spec 015 (voice): the participant pipeline stays engine-side; only capture and
transport vary per surface.
