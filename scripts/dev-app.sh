#!/usr/bin/env bash
# dev-app.sh - wrap the wagner-edge-shell binary in a minimal, ad-hoc-signed .app
# so macOS will grant microphone access. A bare `cargo run` binary never gets a
# TCC mic prompt; only a signed .app bundle with NSMicrophoneUsageDescription does.
#
# Voice adopts the sidecars started by scripts/voice-sidecars.sh, so this app does
# NOT need the bundled sidecar binaries. Re-run after code changes.
#
#   scripts/voice-sidecars.sh start   # once
#   scripts/dev-app.sh                # builds + signs WagnerDev.app
#   open WagnerDev.app                # first launch prompts for the mic - Allow it
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/WagnerDev.app"

echo "[dev-app] building frontend..."
npm --prefix "$ROOT/edge/ui" run build >/dev/null

echo "[dev-app] building wagner-edge-shell (custom-protocol)..."
( cd "$ROOT/edge/shell" && cargo build --features custom-protocol )

BIN="$ROOT/target/debug/wagner-edge-shell"
if [ ! -x "$BIN" ]; then
  echo "binary not found: $BIN" >&2
  exit 1
fi

echo "[dev-app] assembling app bundle..."
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/wagner-edge-shell"
if [ -f "$ROOT/edge/shell/icons/icon.icns" ]; then
  cp "$ROOT/edge/shell/icons/icon.icns" "$APP/Contents/Resources/icon.icns"
fi

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>wagner-edge-shell</string>
  <key>CFBundleIdentifier</key><string>com.marktripoli.wagner-edge</string>
  <key>CFBundleName</key><string>Wagner Edge (dev)</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>icon.icns</string>
  <key>NSMicrophoneUsageDescription</key><string>Wagner uses the microphone for push-to-talk and hands-free voice commands.</string>
</dict>
</plist>
PLIST

ENTS="$(mktemp)"
cat > "$ENTS" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.device.audio-input</key><true/>
</dict>
</plist>
PLIST

echo "[dev-app] ad-hoc signing..."
codesign --force --deep --sign - --entitlements "$ENTS" "$APP"
rm -f "$ENTS"

echo "[dev-app] done -> $APP"
echo "Run:  open \"$APP\"   (first launch prompts for microphone access - click Allow)"
