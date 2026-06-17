---
title: Resume uses default roster + no suite (roster not persisted)
summary: resume_run rebuilds a session against the persisted project_dir but uses the default roster and no suite command, because Run does not persist the hired roster or suite.
tags: [decision, sessions, engine, ceiling]
tier: supporting
---

# Resume defaults

**Decided:** 2026-06-17 during Plan 003 Step 4.

`resume_run` reloads a `Run` from disk, rebuilds the gate + agent pool against the
persisted `project_dir`, and re-enters the goal loop via the shared
`spawn_run_loop` helper (the same one `start_run` uses — they cannot drift).

**Ceiling:** `Run` does not persist the hired **roster** or the **suite command**,
so a resumed session uses `Roster::default_roster()` and `suite_command: None`.
For most runs (default Cipher/Vex org, suite declared by the repo's CLAUDE.md)
this is correct. **Upgrade path:** persist `roster` + `suite_command` on `Run`
(schema additions) to restore custom rosters on resume — do it when a user
actually resumes a custom-roster run and notices.

`prepare_resumed(run)` sets `status = Running` and clears `halt_reason` (a resumed
session runs again; a prior guardrail halt is cleared). Unit-tested in
`commands.rs`. Deeper resume behavior (load → continue loop) is covered by the
host integration tests `reopen_fidelity.rs` / `run_survives_close.rs`.

Related: [[002-run-is-session]], [[003-acceptance-harness-strategy]].
