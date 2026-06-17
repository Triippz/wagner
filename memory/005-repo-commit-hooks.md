---
title: Repo commit hooks (constraints on every commit)
summary: This repo enforces three PreToolUse git hooks — no main commits, no "Claude"/AI in messages, no `git add -A`. Build commits land on feat/autonomous-build.
tags: [decision, process, git, gotcha]
tier: core
---

# Repo commit hooks

**Learned:** 2026-06-17 (the hard way, on commit #1).

Three harness hooks gate every commit. Obey them on every step:

1. **No direct commits to `main`/`master`.** Blocked by
   `block-main-branch-commit-push.sh`. Requires interactive operator permission
   I can't grant unattended. → **All autonomous commits go on
   `feat/autonomous-build`.** Operator fast-forwards `main` when they wake. This
   is a forced deviation from the literal "commit to main" instruction; see
   [[001-autonomous-build-mandate]]. (The hook's other escape, prefixing
   `ALLOW_MAIN_BRANCH_COMMIT=1`, needs operator confirmation and reads as a bypass
   to the auto-classifier — do not use it unattended.)
2. **No "Claude" / AI references in commit messages.** Blocked by
   `commit-msg-check.sh`. This OVERRIDES the global "Co-Authored-By: Claude…"
   convention — omit the co-author trailer entirely in this repo.
3. **No `git add -A` / `git add .`.** Blocked by `require-explicit-staging.sh`.
   Stage explicit file paths: `git add path1 path2 …`.

**Commit recipe that works:**
```
git checkout -b feat/autonomous-build   # once
git add <explicit paths>
git commit -q -F - <<'EOF'
type: [scope] subject (no AI references)
…body…
EOF
```

Related: [[001-autonomous-build-mandate]], [[004-roadmap-and-parallelization]].
