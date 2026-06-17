# Wagner platform — standalone repo root Makefile.
# Repo root IS the platform root (was platform/ in dev-ai-utilities).
# Cargo runs from repo root so rustup honors rust-toolchain.toml.
CARGO_EDGE_HOST := -p wagner-edge-host

.PHONY: dev cargo clippy e2e ts arch typecheck hub verify \
	edge edge-build edge-ui shell

# Launch the live MV hub (Deno + Hono): OIDC + ephemeral discovery (long-running).
dev:
	cd hub && deno task dev

# Build the edge desktop frontend (vite → edge/ui/dist), then launch the Tauri
# shell binary (the assembled engine + gate + tray + UI). This is the live
# desktop app. `--features custom-protocol` makes Tauri serve the embedded
# `frontendDist` via the `tauri://` asset protocol; WITHOUT it `cargo run` is
# "dev mode" (`is_dev() == !cfg!(custom-protocol)`) and loads the `devUrl` vite
# server on :1420 — blank unless that server is up. For hot-reload dev instead,
# `cargo install tauri-cli` and run `cd edge/shell && cargo tauri dev`.
edge: edge-build
	cargo run -p wagner-edge-shell --features custom-protocol

# Build only the edge frontend bundle (vite → edge/ui/dist).
edge-build:
	npm --prefix edge/ui run build

# Headless browser smoke test of the edge UI: spawns vite, drives the console
# through composer → running → inspector → permission via Playwright (the `?mock`
# transport seam), screenshots each, fails on any console error.
edge-ui:
	npm --prefix edge/ui run test:ui

# Edge desktop shell crate: compile + clippy + the resolve-path unit tests.
shell:
	cargo build -p wagner-edge-shell
	cargo clippy -p wagner-edge-shell --all-targets -- -D warnings
	cargo test -p wagner-edge-shell

# Rust edge-host engine: full test suite (unit + integration + gate e2e).
cargo:
	cargo test $(CARGO_EDGE_HOST)

# Rust edge-host engine: clippy lint gate (warnings are errors).
clippy:
	cargo clippy $(CARGO_EDGE_HOST) --all-targets -- -D warnings

# Cross-process gate e2e only, with engine output (spawns the real node gate).
e2e:
	cargo test $(CARGO_EDGE_HOST) --test gate_e2e -- --nocapture

# TypeScript: edge UI + shared reducer + arch-boundary tests (all workspaces).
ts:
	npm run test

# Architecture-boundary tests only (dependency direction).
arch:
	npm run test:arch

# TypeScript type-check across all workspaces (no emit).
typecheck:
	npm run typecheck

# Hub (Deno): OIDC + discovery registry unit tests.
hub:
	cd hub && deno task test

# Full pre-merge gate: clippy → cargo → shell → typecheck → ts →
# edge-frontend build → hub.
verify: clippy cargo shell typecheck ts edge-build hub
