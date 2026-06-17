#!/usr/bin/env bash
# Start/stop the voice STT+TTS sidecars that HttpStt/HttpTts (and `make
# voice-e2e`) expect on 127.0.0.1:8771 / :8772.
#
#   STT  faster-whisper-server  (OpenAI-compatible /v1/audio/transcriptions)
#   TTS  Kokoro-FastAPI         (OpenAI-compatible /v1/audio/speech)
#
# Both run as off-the-shelf CPU Docker images — no Python venv, no model
# vendoring. Models download on first use into named volumes (cached across
# restarts).
#
#   scripts/voice-sidecars.sh start    # pull (if needed) + run + wait ready
#   scripts/voice-sidecars.sh stop     # stop + remove both containers
#   scripts/voice-sidecars.sh status   # show container + port state
#
# ponytail: published images implement the exact wire contract; this script is
# just lifecycle glue. Swap the image tags below if upstream moves.
set -euo pipefail

STT_IMAGE="fedirz/faster-whisper-server:latest-cpu"
TTS_IMAGE="ghcr.io/remsky/kokoro-fastapi-cpu:latest"
STT_NAME="wagner-stt"
TTS_NAME="wagner-tts"
STT_PORT=8771   # host port HttpStt::new("http://127.0.0.1:8771") expects
TTS_PORT=8772   # host port HttpTts::new("http://127.0.0.1:8772") expects
STT_INTERNAL=8000   # faster-whisper-server listens here inside the container
TTS_INTERNAL=8880   # Kokoro-FastAPI listens here inside the container
# Tiny English model: fast + small (~75MB) — enough to prove the pipeline.
STT_MODEL="Systran/faster-whisper-tiny.en"

err() { echo "[voice-sidecars] $*" >&2; }

wait_ready() {
  local name="$1" port="$2" tries=60
  err "waiting for $name on :$port (up to ${tries}s)…"
  for ((i = 0; i < tries; i++)); do
    # Any real HTTP response (curl prints 000 when nothing is listening) means
    # the server is up. Don't `|| echo` — that concatenates onto the -w output.
    local code
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "http://127.0.0.1:${port}/health" 2>/dev/null) || true
    if [[ "$code" =~ ^[1-5][0-9][0-9]$ ]]; then
      err "$name ready (HTTP $code on /health)"
      return 0
    fi
    if ! docker ps --format '{{.Names}}' | grep -q "^${name}$"; then
      err "FAIL: $name container exited early — logs:"
      docker logs --tail 20 "$name" 2>&1 | sed 's/^/    /' >&2
      return 1
    fi
    sleep 1
  done
  err "FAIL: $name not ready after ${tries}s — logs:"
  docker logs --tail 20 "$name" 2>&1 | sed 's/^/    /' >&2
  return 1
}

start() {
  docker rm -f "$STT_NAME" "$TTS_NAME" >/dev/null 2>&1 || true

  err "starting STT ($STT_IMAGE) → :$STT_PORT"
  docker run -d --name "$STT_NAME" \
    -p "127.0.0.1:${STT_PORT}:${STT_INTERNAL}" \
    -e "WHISPER__MODEL=${STT_MODEL}" \
    -v wagner-stt-cache:/root/.cache \
    "$STT_IMAGE" >/dev/null

  err "starting TTS ($TTS_IMAGE) → :$TTS_PORT"
  docker run -d --name "$TTS_NAME" \
    -p "127.0.0.1:${TTS_PORT}:${TTS_INTERNAL}" \
    -v wagner-tts-cache:/root/.cache \
    "$TTS_IMAGE" >/dev/null

  wait_ready "$STT_NAME" "$STT_PORT"
  wait_ready "$TTS_NAME" "$TTS_PORT"
  err "both sidecars up. run: make voice-e2e"
}

stop() {
  docker rm -f "$STT_NAME" "$TTS_NAME" >/dev/null 2>&1 || true
  err "stopped + removed $STT_NAME / $TTS_NAME"
}

status() {
  docker ps -a --filter "name=${STT_NAME}" --filter "name=${TTS_NAME}" \
    --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
}

case "${1:-start}" in
  start) start ;;
  stop) stop ;;
  status) status ;;
  *) err "usage: $0 {start|stop|status}"; exit 2 ;;
esac
