# Wagner platform

The org engineering platform — *edge executes, hub remembers*. One monorepo, two
layers: the capability library (skills/agents/commands, repo root) consumed by
the platform runtime here under `platform/`. See `prd.md` for the vision and
`docs/spec/constitution.md` for the non-negotiables.

## Layout

```
platform/
├── shared/      # the spine: JSON schemas + pure reducers + transport contract
│   ├── schemas/         run, learning, transmission + the wedge-002 channel schemas
│   ├── reducer/         run-reducer (event-sourced spine) + remote-reducer (sessions)
│   └── transport/       EventStreamTransport contract (IPC | P2P)
├── edge/
│   ├── host/    # Rust engine (goal loop, gate, log, memory) + tray + remote
│   │   └── src/  dirs: cli, events, orchestrator, remote, state, tray
│   │            files: lib.rs, memory.rs, permission_server.rs, schema.rs, transmissions.rs
│   └── ui/      # ONE responsive React/TS surface (desktop/web/mobile)
│       ├── store/        carried UI projection reducer
│       ├── surfaces/     unified surface fold + a11y + capability availability
│       └── transport/    IPC + P2P adapters (env-detected)
├── hub/         # Deno + Hono + jose: OIDC + ephemeral discovery registry
│   └── src/{auth,discovery,relay,routes} + app.ts, main.ts
├── tests/architecture/   Article VII dependency-direction guard
└── docs/{adr,spec}       ADRs + constitution/defaults
```

Dependency direction is one-way: `edge` and `hub` depend on `shared`, never the
reverse; nothing outside `platform/` imports it (Article VII, enforced by
`tests/architecture`).

## Status

- **Wedge 001 — shared runs + learnings:** event-sourced run spine + schemas
  landed; hub sync/recall pending (`specs/001-shared-runs-and-learnings`).
- **Wedge 002 — edge surface & remote sessions:** Foundation + US1 (tray-resident
  host + unified surface) + US2 (SSO-gated edge-armed remote attach, hub-side
  degraded) + US3 (remote agency: gated control + repo-scoped dev-context, no
  PTY) — **logic complete and tested**. Live integration points (Tauri shell,
  live iroh endpoint, org relay) are documented and integration-only. See
  `specs/002-edge-surface-and-remote-sessions/` + `quickstart.md`.

## Run the tests

```bash
cargo test                              # edge host (Rust)
npm run test -w @wagner/shared          # spine
npm run test -w @wagner/edge-ui         # surface
npm run test:arch                       # dependency-direction guard
( cd hub && deno test -A )              # hub (Deno)
```

Toolchain: Rust pinned to 1.91 (`rust-toolchain.toml`; iroh 1.0 needs > 1.90);
Node 22 + vitest; Deno 2.7.
