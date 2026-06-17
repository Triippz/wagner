#!/usr/bin/env bash
# Native launcher for the Wagner voice sidecars.
# No Docker — both engines are native binaries.
#
#   STT  whisper-server (whisper.cpp, brew install whisper-cpp)
#        → POST /v1/audio/transcriptions  on 127.0.0.1:8771
#
#   TTS  wagner-tts-sidecar (Rust, built from crate edge/tts-sidecar)
#        → POST /v1/audio/speech          on 127.0.0.1:8772
#
# Usage:
#   scripts/voice-sidecars.sh start    # download models if needed → spawn both → wait ready
#   scripts/voice-sidecars.sh stop     # kill both sidecars
#   scripts/voice-sidecars.sh status   # show running state
#
# Environment:
#   WAGNER_VOICE_HOME   — cache root (default: $HOME/.cache/wagner-voice)
#   WAGNER_REPO_ROOT    — repo root for finding the tts-sidecar binary (auto-detected)
#
set -euo pipefail

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

VOICE_HOME="${WAGNER_VOICE_HOME:-${HOME}/.cache/wagner-voice}"
MODEL_DIR="${VOICE_HOME}/models"
PID_DIR="${VOICE_HOME}/run"

# Auto-detect repo root (the directory that contains this script's parent).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${WAGNER_REPO_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"

STT_PORT=8771
TTS_PORT=8772

# Whisper model
STT_MODEL_FILE="${MODEL_DIR}/ggml-tiny.en.bin"
STT_MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"

# Kokoro ONNX model + voices
TTS_MODEL_FILE="${MODEL_DIR}/model_quantized.onnx"
TTS_MODEL_URL="https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/onnx/model_quantized.onnx"
TTS_VOICES_FILE="${MODEL_DIR}/voices-v1.0.bin"
TTS_VOICES_URL="https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin"

# TTS sidecar binary
TTS_BIN="${REPO_ROOT}/target/release/wagner-tts-sidecar"

# PID files
STT_PID_FILE="${PID_DIR}/stt.pid"
TTS_PID_FILE="${PID_DIR}/tts.pid"
STT_LOG_FILE="${PID_DIR}/stt.log"
TTS_LOG_FILE="${PID_DIR}/tts.log"

# ---------------------------------------------------------------------------
# SHA-256 constants (computed from known-good cached files)
# ---------------------------------------------------------------------------

SHA256_STT_MODEL="921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f"
SHA256_TTS_MODEL="fbae9257e1e05ffc727e951ef9b9c98418e6d79f1c9b6b13bd59f5c9028a1478"
SHA256_TTS_VOICES="bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log() { echo "[voice-sidecars] $*" >&2; }

# Verify SHA-256 of a file; delete and exit 1 on mismatch.
verify_sha256() {
    local file="$1" expected="$2" label="$3"
    log "verifying SHA-256 of ${label}…"
    local actual
    actual="$(shasum -a 256 "${file}" | awk '{print $1}')"
    if [[ "${actual}" != "${expected}" ]]; then
        log "ERROR: SHA-256 mismatch for ${label}"
        log "  expected: ${expected}"
        log "  actual:   ${actual}"
        rm -f "${file}"
        exit 1
    fi
    log "${label} SHA-256 OK"
}

# Download a file if it does not exist (or is 0 bytes). Atomic: downloads to
# a .tmp file and moves into place only on success; a RETURN trap ensures
# partial/interrupted downloads never leave a stale file.
download_if_missing() {
    local url="$1" dest="$2" label="$3" expected_sha="$4"
    if [[ -f "${dest}" && -s "${dest}" ]]; then
        log "${label} already cached at ${dest}"
        return 0
    fi
    log "downloading ${label} → ${dest}"
    mkdir -p "$(dirname "${dest}")"
    local tmp="${dest}.tmp"
    # Remove any leftover .tmp from a previous interrupted download.
    rm -f "${tmp}"
    # Trap ensures the .tmp is cleaned up if this function exits for any reason.
    trap 'rm -f "${tmp}"' RETURN
    # -f: fail on server error, -L: follow redirects, -C -: resume if partial
    curl -fL -C - --progress-bar -o "${tmp}" "${url}"
    mv "${tmp}" "${dest}"
    log "${label} download complete ($(du -h "${dest}" | cut -f1))"
    verify_sha256 "${dest}" "${expected_sha}" "${label}"
}

# Return 0 if a process with the given PID is running.
pid_alive() {
    local pid="$1"
    [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null
}

# Read a PID file; echo the PID if the process is alive, else nothing.
read_pid() {
    local pidfile="$1"
    if [[ -f "${pidfile}" ]]; then
        local pid
        pid="$(cat "${pidfile}")"
        if pid_alive "${pid}"; then
            echo "${pid}"
        fi
    fi
}

# Wait for a TCP port to accept connections (process-based, not /health).
# The TTS sidecar has no /health route; we poll the port directly.
wait_port() {
    local label="$1" port="$2" pidfile="$3" tries=60
    log "waiting for ${label} on :${port} (up to ${tries}s)…"
    for ((i = 0; i < tries; i++)); do
        # nc -z exits 0 when the port accepts a connection.
        if nc -z 127.0.0.1 "${port}" 2>/dev/null; then
            log "${label} accepting connections on :${port}"
            return 0
        fi
        # Bail early if the process has already died.
        local pid
        pid="$(read_pid "${pidfile}")" || true
        if [[ -z "${pid}" ]]; then
            log "FAIL: ${label} process exited before becoming ready"
            return 1
        fi
        sleep 1
    done
    log "FAIL: ${label} not ready after ${tries}s"
    return 1
}

# Wait for whisper-server's /health endpoint to return HTTP 200.
# whisper-server returns non-200 while the model is loading, so we keep
# polling until we see exactly 200.
wait_http_health() {
    local label="$1" port="$2" pidfile="$3" tries=60
    log "waiting for ${label} /health on :${port} (up to ${tries}s)…"
    for ((i = 0; i < tries; i++)); do
        local code
        code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 \
            "http://127.0.0.1:${port}/health" 2>/dev/null)" || true
        if [[ "${code}" == "200" ]]; then
            log "${label} ready (HTTP 200 on /health)"
            return 0
        fi
        local pid
        pid="$(read_pid "${pidfile}")" || true
        if [[ -z "${pid}" ]]; then
            log "FAIL: ${label} process exited before /health was ready"
            return 1
        fi
        sleep 1
    done
    log "FAIL: ${label} /health not ready after ${tries}s"
    return 1
}

# Kill a process by PID file (best-effort).
kill_pid_file() {
    local label="$1" pidfile="$2"
    if [[ -f "${pidfile}" ]]; then
        local pid
        pid="$(cat "${pidfile}")"
        if pid_alive "${pid}"; then
            log "stopping ${label} (pid ${pid})"
            kill "${pid}" 2>/dev/null || true
        fi
        rm -f "${pidfile}"
    fi
}

# ---------------------------------------------------------------------------
# start
# ---------------------------------------------------------------------------

start() {
    # 0. Already-running guard: if both sidecars are live, do nothing.
    local existing_stt existing_tts
    existing_stt="$(read_pid "${STT_PID_FILE}")" || true
    existing_tts="$(read_pid "${TTS_PID_FILE}")" || true
    if [[ -n "${existing_stt}" && -n "${existing_tts}" ]]; then
        log "sidecars already running (stt pid=${existing_stt}, tts pid=${existing_tts})"
        log "to restart: $0 stop && $0 start"
        return 0
    fi

    # 1. Ensure whisper-server is on PATH.
    if ! command -v whisper-server >/dev/null 2>&1; then
        log "ERROR: whisper-server not found."
        log "       Install with: brew install whisper-cpp"
        exit 1
    fi

    # 2. Ensure TTS sidecar binary exists (build if missing).
    if [[ ! -x "${TTS_BIN}" ]]; then
        log "wagner-tts-sidecar binary not found; building…"
        cargo build -p wagner-tts-sidecar --release \
            --manifest-path "${REPO_ROOT}/Cargo.toml"
        log "build complete: ${TTS_BIN}"
    else
        log "TTS sidecar binary: ${TTS_BIN}"
    fi

    # 3. Download models if missing (idempotent, resumable, SHA-256 verified).
    download_if_missing "${STT_MODEL_URL}" "${STT_MODEL_FILE}" "ggml-tiny.en.bin (STT)"     "${SHA256_STT_MODEL}"
    download_if_missing "${TTS_MODEL_URL}"    "${TTS_MODEL_FILE}"  "model_quantized.onnx (TTS)" "${SHA256_TTS_MODEL}"
    download_if_missing "${TTS_VOICES_URL}"   "${TTS_VOICES_FILE}" "voices-v1.0.bin (TTS)"      "${SHA256_TTS_VOICES}"

    # 4. Create run dir for PID/log files.
    mkdir -p "${PID_DIR}"

    # 5. Stop any stale instances.
    kill_pid_file "whisper-server" "${STT_PID_FILE}"
    kill_pid_file "wagner-tts-sidecar" "${TTS_PID_FILE}"

    # 6. Spawn whisper-server (STT) in the background.
    log "spawning whisper-server → :${STT_PORT}"
    whisper-server \
        --host 127.0.0.1 \
        --port "${STT_PORT}" \
        --inference-path /v1/audio/transcriptions \
        --model "${STT_MODEL_FILE}" \
        --threads 4 \
        >"${STT_LOG_FILE}" 2>&1 &
    local stt_pid=$!
    echo "${stt_pid}" >"${STT_PID_FILE}"
    log "whisper-server pid ${stt_pid}"

    # 7. Spawn wagner-tts-sidecar (TTS) in the background.
    log "spawning wagner-tts-sidecar → :${TTS_PORT}"
    "${TTS_BIN}" \
        --port "${TTS_PORT}" \
        --model "${TTS_MODEL_FILE}" \
        --voices "${TTS_VOICES_FILE}" \
        >"${TTS_LOG_FILE}" 2>&1 &
    local tts_pid=$!
    echo "${tts_pid}" >"${TTS_PID_FILE}"
    log "wagner-tts-sidecar pid ${tts_pid}"

    # 8. Wait for readiness.
    wait_http_health "whisper-server" "${STT_PORT}" "${STT_PID_FILE}"
    wait_port        "wagner-tts-sidecar" "${TTS_PORT}" "${TTS_PID_FILE}"

    log "both sidecars up. run: make voice-e2e"
}

# ---------------------------------------------------------------------------
# stop
# ---------------------------------------------------------------------------

stop() {
    kill_pid_file "whisper-server"      "${STT_PID_FILE}"
    kill_pid_file "wagner-tts-sidecar"  "${TTS_PID_FILE}"
    log "sidecars stopped"
}

# ---------------------------------------------------------------------------
# status
# ---------------------------------------------------------------------------

status() {
    local stt_pid tts_pid
    stt_pid="$(read_pid "${STT_PID_FILE}")" || true
    tts_pid="$(read_pid "${TTS_PID_FILE}")" || true

    if [[ -n "${stt_pid}" ]]; then
        echo "whisper-server        running  pid=${stt_pid}  :${STT_PORT}"
    else
        echo "whisper-server        stopped"
    fi

    if [[ -n "${tts_pid}" ]]; then
        echo "wagner-tts-sidecar    running  pid=${tts_pid}  :${TTS_PORT}"
    else
        echo "wagner-tts-sidecar    stopped"
    fi
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------

case "${1:-start}" in
    start)  start  ;;
    stop)   stop   ;;
    status) status ;;
    *) log "usage: $0 {start|stop|status}"; exit 2 ;;
esac
