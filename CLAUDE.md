# Repository Guidelines

## Agent File Sync Rule

`AGENTS.md` and `CLAUDE.md` are lockstep mirrors. Any edit to one must be made to the other in the same change. Before handing off agent-guidance changes, run `cmp -s CLAUDE.md AGENTS.md`; a nonzero exit means the change is incomplete.

## Project Structure & Module Organization

Wagner is a monorepo for a local-first personal OS: edge-local agent/workflow execution, voice/text interaction, vault knowledge graph, dedicated workspaces, and a small hub for cloud-connected sync/coordination. `edge/host` is the Rust run engine, `edge/shell` is the Tauri shell, and `edge/ui` is the React/Vite surface. `shared/` contains TypeScript schemas, pure reducers, and transport contracts. `hub/` is the Deno/Hono service. Architecture guards live in `tests/architecture`; ADRs and platform rules live in `docs/adr` and `docs/spec`; feature plans live in `specs/`.

## Product Vision

`VISION.md` is the product source of truth. Do not frame Wagner as only a coding tool, IDE clone, or passive voice bot. It is a voice-first and text-equal personal OS for agents, deterministic workflows, knowledge, search/research, media/artifact generation, productivity connectors, and heavy workspaces such as coding. The desired feel is a serious smart system for daily work: cinematic, grounded, precise, local-first, extensible, and plain-spoken.

## Build, Test, and Development Commands

Run `make dev-setup` after cloning to verify Rust, Node, Deno, and Docker. Use `npm install` for workspace dependencies. Key checks are `make cargo` for Rust engine tests, `make clippy` for Rust linting, `make ts` for TypeScript tests, `make typecheck`, `make hub` for Deno tests, and `make verify` for the pre-merge gate. Use `make edge` for the desktop app and `make run` / `make down` for the full local stack.

## Coding Style & Naming Conventions

Rust uses edition 2021 and must pass `cargo clippy --all-targets -- -D warnings`. TypeScript is strict and module-based; keep `shared/` pure, with no UI or I/O dependencies. Hub code follows `hub/deno.json`, including semicolons and 100-column lines. Preserve the one-way dependency rule: edge and hub may depend on `shared`, but not the reverse.

## Testing Guidelines

Prefer targeted tests while developing, then run `make verify` before a PR. Place Rust integration tests under the relevant crate `tests/` directory. Keep Vitest files named `*.test.ts`, and keep hub tests under `hub/tests/{unit,contract,e2e}`. Update schemas and schema tests together when payload shapes change.

## Commit & Pull Request Guidelines

Commit messages use `type: [scope] description`, such as `feat: [voice] add sidecar launcher`. Allowed types are `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, and `chore`. Do not use `type(scope):` or AI tool attribution. PRs should include a summary, linked spec or issue, verification commands, and screenshots for UI changes.

## Agent-Specific Notes

When running shell commands as a Codex agent, prefix commands with `rtk`. Keep secrets out of commits; hub OIDC settings belong in the local environment. Default sync behavior must respect the privacy boundary in `docs/spec/constitution.md`: curated knowledge and metadata may sync, but raw code, secrets, private files, and full transcripts must remain local unless a future feature explicitly gates per-item sharing.
