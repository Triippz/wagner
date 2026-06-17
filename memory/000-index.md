---
title: Memory index (Map of Content)
summary: Index of all autonomous-build decision notes; the entry point after a compaction.
tags: [index, moc]
tier: core
---

# Memory — decision index

Obsidian/Karpathy-style decision log for the autonomous Wagner build. One note =
one decision/fact. Read [[001-autonomous-build-mandate]] first, then this index.
The live build state lives in `../build-context.md`.

## Notes

- [[001-autonomous-build-mandate]] — what the operator authorized (full build, commit to main, no questions).
- [[002-run-is-session]] — a Run IS a session; events already carry run_id; reducer-only keying.
- [[003-acceptance-harness-strategy]] — how "done" is gated without a native E2E (fake agent + mock transport).
- [[004-roadmap-and-parallelization]] — phase DAG and which lanes run in parallel.
- [[005-repo-commit-hooks]] — git hook constraints; commits land on feat/autonomous-build.
- [[006-resume-defaults]] — resume_run uses default roster/no suite (not persisted on Run).
- [[007-worktree-lane-integration]] — worktree lanes branch from base; integrate via conflict-resolution.
- [[008-n0-relays]] — vault sync uses n0 public relays (RelayMode::Default) via vault_relay_mode() seam.

## Conventions

- Filename: `NNN-kebab-slug.md`, NNN zero-padded sequential.
- Frontmatter: `title`, `summary` (≤200ch), `tags`, `tier` (core/supporting/peripheral).
- Link related notes with `[[NNN-slug]]`. Add new notes to this index.
