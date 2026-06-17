#!/usr/bin/env bash
# stage-voice-binaries.sh — build wagner-tts-sidecar in release, then copy
# both voice sidecar binaries into edge/shell/binaries/ with the target-triple
# suffix that Tauri's externalBin expects.
#
# Usage: bash scripts/stage-voice-binaries.sh [--target <triple>]
#
# Overridable environment variables:
#   WHISPER_SERVER_BIN   path to the whisper-server binary
#                        (default: $(command -v whisper-server))
#
# After this script succeeds, run:
#   cd edge/shell && cargo tauri build
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARIES_DIR="$REPO_ROOT/edge/shell/binaries"

# Resolve target triple from rustc unless --target is passed.
TARGET=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$TARGET" ]]; then
  TARGET="$(rustc -vV 2>/dev/null | grep '^host:' | awk '{print $2}')"
  if [[ -z "$TARGET" ]]; then
    echo "ERROR: could not resolve host target triple via rustc -vV" >&2
    exit 1
  fi
fi

# S2: Validate $TARGET against a target-triple pattern before using it in
# filesystem paths; prevents path traversal via a crafted --target argument.
if [[ ! "$TARGET" =~ ^[a-zA-Z0-9_]+-[a-zA-Z0-9_]+-[a-zA-Z0-9_.+-]+$ ]]; then
  echo "ERROR: '$TARGET' does not look like a valid target triple (e.g. aarch64-apple-darwin)" >&2
  exit 1
fi

echo "[stage-voice-binaries] target triple: $TARGET"

# Locate whisper-server.
WHISPER_SERVER_BIN="${WHISPER_SERVER_BIN:-$(command -v whisper-server 2>/dev/null || true)}"
if [[ -z "$WHISPER_SERVER_BIN" || ! -x "$WHISPER_SERVER_BIN" ]]; then
  echo "ERROR: whisper-server not found on PATH and WHISPER_SERVER_BIN is unset." >&2
  echo "       Install via Homebrew: brew install whisper-cpp" >&2
  echo "       Or set WHISPER_SERVER_BIN=/path/to/whisper-server" >&2
  exit 1
fi
echo "[stage-voice-binaries] whisper-server: $WHISPER_SERVER_BIN"

# Build wagner-tts-sidecar in release.
echo "[stage-voice-binaries] building wagner-tts-sidecar (release)…"
cargo build --release -p wagner-tts-sidecar
TTS_BIN="$REPO_ROOT/target/release/wagner-tts-sidecar"
if [[ ! -x "$TTS_BIN" ]]; then
  echo "ERROR: expected release binary not found at $TTS_BIN" >&2
  exit 1
fi
echo "[stage-voice-binaries] TTS binary: $TTS_BIN"

# Stage both binaries with target-triple suffix.
mkdir -p "$BINARIES_DIR"
cp "$WHISPER_SERVER_BIN" "$BINARIES_DIR/whisper-server-$TARGET"
cp "$TTS_BIN"            "$BINARIES_DIR/wagner-tts-sidecar-$TARGET"
chmod +x "$BINARIES_DIR/whisper-server-$TARGET"
chmod +x "$BINARIES_DIR/wagner-tts-sidecar-$TARGET"

echo "[stage-voice-binaries] staged:"
echo "  $BINARIES_DIR/whisper-server-$TARGET"
echo "  $BINARIES_DIR/wagner-tts-sidecar-$TARGET"
echo "[stage-voice-binaries] done — run: cd edge/shell && cargo tauri build"
