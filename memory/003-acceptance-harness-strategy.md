---
title: Acceptance harness strategy (no native E2E on macOS)
summary: "Done" is gated by real-engine tests (fake AgentPool) + real-UI journey (mock transport seam), wired into `make accept`. Native-shell E2E is impractical on macOS.
tags: [decision, testing, acceptance, harness]
tier: core
---

# Acceptance harness strategy

**Decided:** 2026-06-17 after surveying the test surface.

**No true native-shell E2E.** Tauri's `tauri-driver` does not support macOS, so
driving the real window/tray/IPC end-to-end can't be stood up reliably. Not worth
fighting for an overnight run.

**What we gate on instead — two real layers, deterministic:**

1. **Engine acceptance (Rust).** `AgentPool` is a trait
   (`edge/host/src/orchestrator/run_loop.rs:24`); `edge/host/tests/goal_loop.rs`
   already drives the *real* `run_goal` loop with a fake agent — no live CLI.
   New session features (resume/concurrency/add_goal/list) get acceptance tests
   at this engine+command level, run by `make cargo` (already in `verify`).
2. **UI journey (browser).** `edge/ui/scripts/ui-smoke.mjs` drives the real React
   UI through the `?mock` transport seam (`window.__wagner.push(channel, payload)`)
   with Playwright, screenshotting stages and failing on console errors. Extend it
   into a full session-lifecycle journey.

**Gate wiring:**
- `make verify` (fast, per-step) = clippy cargo shell typecheck ts edge-build hub.
- `make accept` (full, per-phase) = `verify` + `edge-ui` (the UI journey).
  The UI journey is promoted into the gate (it wasn't in `verify` before).

**Why this is enough for unattended work:** acceptance tests encode each plan's
acceptance criteria, so "green" objectively means "feature works" without the
operator. Related: [[002-run-is-session]], [[001-autonomous-build-mandate]].
