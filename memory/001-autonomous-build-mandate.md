---
title: Autonomous build mandate
summary: Operator authorized a full autonomous overnight build of the entire Wagner product, committing each green step to main, no questions.
tags: [decision, process, autonomy]
tier: core
---

# Autonomous build mandate

**Decided:** 2026-06-17 by the operator (Mark), then sleeping.

The operator authorized building the **entire Wagner product** (all phases:
Sessions, Vault, Graph, Sync, Hub, Voice) autonomously and unattended, with these
explicit instructions:

- Start with an **acceptance/E2E test harness**, then build full-on.
- **Commit each green step to `main`** (never red). Backups to `.backups/` before
  large edits.
- **Do not ask questions** during the build — decide from the locked plan +
  sensible defaults, record decisions here.
- Maintain `../build-context.md` (live state) + this `memory/` dir so
  **autocompaction never loses context**.
- Use Karpathy/Obsidian linked-markdown techniques for the context layer (we also
  dogfood the very Vault model we're building).

**Why:** maximize unattended throughput while staying safe to review (atomic green
commits) and resumable across compactions.

**How to apply:** every step ends with: green `make verify` → commit → update
`build-context.md`. New decisions become `memory/NNN-*.md`. See
[[004-roadmap-and-parallelization]] for scope order.
