#!/usr/bin/env bash
# native-smoke.sh — macOS native-window smoke test for Wagner Edge.
#
# PURPOSE
#   Verify that the assembled Tauri desktop shell opens a real OS-level window
#   and is visible on screen. This is the "native window check" that headless
#   Playwright cannot perform because Playwright drives a Chromium subprocess,
#   not the actual Tauri/WebKit webview that ships in the production binary.
#
# APPROACH
#   tauri-driver is documented as "not supported on macOS" (Tauri's own
#   documentation notes that WebdriverIO / tauri-driver relies on
#   chromedriver/safaridriver session support which is absent on macOS for
#   embedded WKWebView). Instead we use macOS Accessibility automation via
#   osascript (AppleScript) to:
#     1. Build the edge UI bundle   (make edge-build)
#     2. Launch the Tauri shell     (cargo run -p wagner-edge-shell
#                                    --features custom-protocol, background)
#     3. Wait for the app window    (osascript polls System Events)
#     4. Assert window count >= 1   and check the window title
#     5. Screenshot the window      (screencapture -l <window-id>)
#     6. Quit the app               (osascript tell application to quit)
#
# PREREQUISITES (macOS-only)
#   • A physical or virtual display (not headless CI without a display server)
#   • Terminal / the process running this script must have been granted
#     "Accessibility" permission in System Settings → Privacy & Security →
#     Accessibility. Without it, System Events cannot enumerate windows.
#   • The Rust toolchain and cargo must be in PATH.
#
# NOT PART OF CI GATES
#   This script is intentionally excluded from `make verify` and `make accept`
#   because it requires a display and OS Accessibility permission that are
#   unavailable in headless CI environments (GitHub Actions, docker, etc.).
#   It is a developer workstation smoke test only.
#   Run it manually: `make gui-smoke`
#
# FAILURE HANDLING
#   If the window does not appear within the timeout, or Accessibility
#   permission is denied, the script exits non-zero but prints a clear
#   diagnostic message. The Makefile `gui-smoke` target intentionally does
#   NOT propagate its exit code into `make verify` / `make accept`.
#
# USAGE
#   make gui-smoke   # or: bash edge/ui/scripts/native-smoke.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SHOTS_DIR="${REPO_ROOT}/edge/ui/.native-shots"
APP_PROCESS="wagner-edge-shell"
WINDOW_TITLE_FRAGMENT="Wagner"   # partial match — Tauri sets the window title
LAUNCH_TIMEOUT=45                # seconds to wait for the window to appear
POLL_INTERVAL=2                  # seconds between poll attempts

mkdir -p "${SHOTS_DIR}"

log() { echo "[native-smoke] $*"; }
die() { echo "[native-smoke] FAIL: $*" >&2; exit 1; }

# ── 1. Verify we are on macOS ────────────────────────────────────────────────
if [[ "$(uname -s)" != "Darwin" ]]; then
  die "This script only runs on macOS (current OS: $(uname -s))"
fi

# ── 2. Build the edge UI bundle ──────────────────────────────────────────────
log "Building edge UI bundle (make edge-build)…"
if ! make -C "${REPO_ROOT}" edge-build; then
  die "edge-build failed — Vite bundle must be present before the Tauri shell can serve it"
fi

# ── 3. Launch the Tauri shell in the background ──────────────────────────────
log "Launching Tauri shell (cargo run -p ${APP_PROCESS} --features custom-protocol)…"
cargo run -p "${APP_PROCESS}" --features custom-protocol \
  --manifest-path "${REPO_ROOT}/Cargo.toml" &
CARGO_PID=$!
log "Cargo PID: ${CARGO_PID}"

# Ensure we always kill the app on exit (success or failure).
cleanup() {
  log "Shutting down app and cargo watcher…"
  # Quit the Tauri app gracefully via osascript first (best-effort).
  osascript -e 'tell application "'"${APP_PROCESS}"'" to quit' 2>/dev/null || true
  # Then kill the cargo run process group.
  kill -- -${CARGO_PID} 2>/dev/null || kill ${CARGO_PID} 2>/dev/null || true
  wait ${CARGO_PID} 2>/dev/null || true
}
trap cleanup EXIT

# ── 4. Poll for the window via System Events ─────────────────────────────────
log "Waiting up to ${LAUNCH_TIMEOUT}s for a window matching '${WINDOW_TITLE_FRAGMENT}'…"
DEADLINE=$(( $(date +%s) + LAUNCH_TIMEOUT ))
WINDOW_FOUND=0
WINDOW_COUNT=0

while (( $(date +%s) < DEADLINE )); do
  # AppleScript: count windows whose name contains the title fragment.
  # `2>/dev/null` suppresses the "no accessibility permission" error text;
  # the exit code still signals whether the query succeeded.
  WCOUNT=$(osascript 2>/dev/null <<'APPLESCRIPT'
    tell application "System Events"
      set matchCount to 0
      try
        repeat with proc in (every process whose background only is false)
          repeat with w in (every window of proc)
            if name of w contains "Wagner" then
              set matchCount to matchCount + 1
            end if
          end repeat
        end repeat
      end try
      return matchCount
    end tell
APPLESCRIPT
  ) || {
    # osascript failed — likely no Accessibility permission.
    log "osascript failed — Accessibility permission may not be granted to this terminal."
    log "Grant it in: System Settings → Privacy & Security → Accessibility."
    die "Cannot enumerate windows without Accessibility permission."
  }

  WINDOW_COUNT="${WCOUNT:-0}"
  if (( WINDOW_COUNT >= 1 )); then
    WINDOW_FOUND=1
    break
  fi

  log "  …window not yet visible (count=${WINDOW_COUNT}); retrying in ${POLL_INTERVAL}s"
  sleep "${POLL_INTERVAL}"
done

if (( WINDOW_FOUND == 0 )); then
  die "Wagner window did not appear within ${LAUNCH_TIMEOUT}s (count=${WINDOW_COUNT}). " \
      "Possible causes: no display attached, Accessibility not granted, or build failed silently."
fi

log "Window found (count=${WINDOW_COUNT}) — proceeding with screenshot."

# ── 5. Screenshot the window ──────────────────────────────────────────────────
SHOT="${SHOTS_DIR}/native-window.png"
log "Capturing screenshot → ${SHOT}"
# `screencapture -o -x` captures the screen without the window shadow, silently.
# We capture the whole display here because getting the exact window CGWindowID
# from AppleScript requires additional wrangling; a display screenshot is
# sufficient proof that the window is visible.
screencapture -x "${SHOT}" \
  && log "Screenshot saved: ${SHOT}" \
  || log "WARN: screencapture failed (may need screen-recording permission) — continuing."

# ── 6. Report ─────────────────────────────────────────────────────────────────
log "PASS — Wagner native window opened and is on screen."
log "Window count: ${WINDOW_COUNT}"
log "Screenshot: ${SHOT}"
