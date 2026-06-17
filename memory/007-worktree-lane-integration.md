---
title: Worktree lanes branch from base — integrate via conflict-resolution
summary: Agent tool isolation:worktree branches the lane from repo base (82990fc), not feat/autonomous-build, so lanes lack in-progress work; integrate by merging and resolving conflicts (keep current code, add the lane's new feature).
tags: [decision, process, parallelization, git, gotcha]
tier: core
---

# Worktree lanes branch from base

**Learned:** 2026-06-17 integrating lane/graph + lane/sync.

The Agent tool's `isolation: worktree` creates the lane's branch from the repo
**base commit (`82990fc`)**, NOT from the current `feat/autonomous-build` HEAD.
So a lane is built without any of the in-progress autonomous work.

**Consequence:**
- Lanes that add only NEW files (Voice `edge/host/src/voice/**`, devex hub/docker
  files) merge cleanly (only `lib.rs`/`Cargo.toml`/`Makefile` one-line overlaps).
- Lanes that EDIT files the main loop also changed (graph → `memory.rs`/`App.tsx`;
  sync → `vault/mod.rs`/`vault/linker.rs`) CONFLICT and need real resolution.

**Integration recipe that works** (proven on graph): delegate to a fresh agent
working DIRECTLY on `feat/autonomous-build` (no worktree). `git merge --no-ff
--no-commit lane/X`; resolve each conflict by KEEPING the current branch's richer
code and ADDING the lane's genuinely-new additive parts (discard duplicate
DDL/scaffolding the current branch already has); `make verify` + `make accept`
both exit 0; explicit-path staging; commit. Never leave conflict markers / a red
tree — abort and report if intractable.

Related: [[004-roadmap-and-parallelization]], [[005-repo-commit-hooks]].
