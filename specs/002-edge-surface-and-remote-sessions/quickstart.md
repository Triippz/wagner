# Quickstart — Edge Surface & Remote Sessions

End-to-end: arm the host, attach from a phone, answer a permission, run `git
diff`, read a file. (Steps marked **live** need the desktop Tauri shell + a
configured org relay — see each task's integration note. The logic each step
relies on is unit-tested headlessly.)

## 0. Prerequisites

- The edge host running (desktop app — `platform/edge/host` + the Tauri shell).
- The hub running with OIDC configured (env only, D-SEC-2):

  ```bash
  cd platform/hub
  export OIDC_ALLOWED_ISSUERS="https://accounts.google.com,https://oauth.id.jumpcloud.com/"
  export OIDC_CLIENT_ID="wagner-hub-client"
  export OIDC_ALLOWED_DOMAINS="adyton.io"
  export OIDC_ISSUER_JWKS="https://accounts.google.com=https://www.googleapis.com/oauth2/v3/certs"
  deno task dev          # serves on :8787
  ```

## 1. Arm the host (edge-only)

On the desktop machine, choose **Arm for remote**. The host advertises its iroh
endpoint and registers its NodeId + a short-lived ticket with the hub under your
verified identity, emitting `remote.armed`. Arming is **edge-only** — no remote
peer can arm your host (SC-203). Closing the window keeps the host + endpoint
alive in the tray (FR-101).

## 2. Attach from a phone

On a second device, sign in with org SSO (OIDC) and open your armed host. The
client resolves the ticket (`POST /v1/discovery/resolve`, owner-only) and
attaches over iroh — direct if possible, else the org-run relay (**live**). The
run view folds the **same** event stream you'd see locally (FR-002).

With the host **not** reachable you can still browse shared learnings and recall
(hub-side); host-side capabilities show *unavailable, with a reason* (FR-204).

## 3. Answer a permission

When the run hits a gated tool, a **needs-you** prompt appears (and raised a tray
notification on the desktop within 5 s). Answer **Allow/Deny** from the phone —
it routes through the **same gate** a local answer does (FR-004); the decision is
identical regardless of origin (SC-202), logged + attributed to you (SC-002).

## 4. Run `git diff` (dev-context command)

Run a non-interactive command (e.g. `git diff`). Output streams back to your
device as frames; the command + exit are logged, **the output is not** (F-1). No
PTY is allocated — for an interactive shell, use `ssh`/`tmux` over a tunnel
(tier ③, not built into Wagner; an interactive-shell channel is refused).

## 5. Read a file (repo-scoped)

Read a file or list the tree. Paths are canonicalised and **default-denied**
outside the repo root — `../`, symlink escapes, and out-of-repo `~/.ssh` /
`.env` are refused and logged as `dev_context.refused` (FR-303, CL-202). File
contents reach your device over P2P, **never the hub** (SC-006).

## Run the tests

```bash
# edge host (Rust)
cd platform && cargo test
# shared spine + edge surface (vitest)
npm run test -w @wagner/shared && npm run test -w @wagner/edge-ui
# architecture guard
npm run test:arch
# hub (Deno)
cd hub && deno test -A
```
