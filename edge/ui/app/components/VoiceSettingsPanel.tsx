import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { cmd } from "../bridge";
import type { ModelState } from "../bridge";

interface ModelStatus {
  stt: ModelState;
  tts: ModelState;
}

// Progress payload from the `wagner://voice-download` channel.
// Rust emits three model ids: "stt" | "tts_model" | "tts_voices".
// Both TTS files map to the single "tts" UI row; we show the latest
// observed state for either file (last-write wins is fine: they
// sequence stt → tts_model → tts_voices, so the row stays "downloading"
// until both are ready and the final tts_voices event arrives as "ready").
interface DownloadProgressPayload {
  model: "stt" | "tts_model" | "tts_voices";
  state: ModelState;
  received: number;
  total: number;
}

/** Map the three Rust model ids to the two UI row keys. */
function uiRow(model: DownloadProgressPayload["model"]): "stt" | "tts" {
  return model === "stt" ? "stt" : "tts";
}

type MockHandle = { on: (ch: string, cb: (p: unknown) => void) => () => void };

const useMock =
  typeof window !== "undefined" &&
  new URLSearchParams(window.location.search).has("mock");

function stateLabel(s: ModelState): string {
  switch (s) {
    case "absent":      return "Not downloaded";
    case "downloading": return "Downloading";
    case "verifying":   return "Verifying";
    case "ready":       return "Ready";
    case "failed":      return "Failed";
  }
}

function stateTone(s: ModelState): string {
  switch (s) {
    case "ready":       return "ready";
    case "downloading":
    case "verifying":   return "progress";
    case "failed":      return "failed";
    default:            return "absent";
  }
}

interface ModelRowProps {
  label: string;
  state: ModelState;
  received: number;
  total: number;
}

function ModelRow({ label, state, received, total }: ModelRowProps) {
  const pct = state === "downloading" && total > 0
    ? Math.min(100, Math.round((received / total) * 100))
    : null;
  return (
    <div className="voice-model-row">
      <span className="voice-model-label">{label}</span>
      <span className="voice-model-state" data-tone={stateTone(state)}>
        {stateLabel(state)}
        {pct !== null && ` ${pct}%`}
      </span>
      {state === "downloading" && total > 0 && (
        <div className="voice-model-bar">
          <i style={{ width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}

interface Props {
  onClose: () => void;
}

export function VoiceSettingsPanel({ onClose }: Props) {
  const [status, setStatus] = useState<ModelStatus>({ stt: "absent", tts: "absent" });
  const [progress, setProgress] = useState<Record<"stt" | "tts", { received: number; total: number }>>({
    stt: { received: 0, total: 0 },
    tts: { received: 0, total: 0 },
  });
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Push-to-talk test (015 US1, M2a): hold to capture, release to transcribe.
  const [recording, setRecording] = useState(false);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [pttError, setPttError] = useState<string | null>(null);

  // Fetch initial status on mount — best-effort (non-Tauri/mock → stays absent).
  useEffect(() => {
    let alive = true;
    cmd.voiceModelsStatus()
      .then((s) => { if (alive) setStatus({ stt: s.stt, tts: s.tts }); })
      .catch(() => {}); // non-Tauri environment: leave default "absent"
    return () => { alive = false; };
  }, []);

  // Subscribe to live download progress.
  //
  // Mock path: window.__wagner.on() side-channel (mirrors VaultPanel).
  // Native path: Tauri listen() on the same channel name; unlisten is the cleanup.
  //
  // Both paths call uiRow() to map "tts_model"/"tts_voices" → the "tts" UI row.
  useEffect(() => {
    if (useMock) {
      const handle = (window as unknown as { __wagner?: MockHandle }).__wagner;
      if (!handle?.on) return;
      return handle.on("wagner://voice-download", (payload) => {
        const p = payload as DownloadProgressPayload;
        const row = uiRow(p.model);
        setStatus((prev) => ({ ...prev, [row]: p.state }));
        setProgress((prev) => ({
          ...prev,
          [row]: { received: p.received, total: p.total },
        }));
      });
    }
    // Native Tauri build: subscribe to the Rust-emitted progress channel.
    // listen() returns a promise that resolves to the unlisten function.
    let unlisten: (() => void) | null = null;
    listen<DownloadProgressPayload>("wagner://voice-download", (event) => {
      const p = event.payload;
      const row = uiRow(p.model);
      setStatus((prev) => ({ ...prev, [row]: p.state }));
      setProgress((prev) => ({
        ...prev,
        [row]: { received: p.received, total: p.total },
      }));
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  const allReady = status.stt === "ready" && status.tts === "ready";
  const anyFailed = status.stt === "failed" || status.tts === "failed";

  function handleDownload() {
    setDownloading(true);
    setError(null);
    cmd.voiceDownloadModels()
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setDownloading(false));
  }

  // Optimistic recording flag: a quick tap may race the async start (the mic
  // isn't open yet on release) and harmlessly report "no capture in progress" —
  // a real hold (hundreds of ms) completes start before release. Fine for a test.
  function startPtt() {
    if (recording) return;
    setPttError(null);
    setRecording(true);
    cmd.voicePttStart().catch((e: unknown) => {
      setRecording(false);
      setPttError(String(e));
    });
  }

  function stopPtt() {
    if (!recording) return;
    setRecording(false);
    cmd.voicePttStop()
      .then((text) => setTranscript(text))
      .catch((e: unknown) => setPttError(String(e)));
  }

  const showDownloadButton =
    !allReady &&
    status.stt !== "downloading" && status.tts !== "downloading" &&
    status.stt !== "verifying" && status.tts !== "verifying";

  return (
    <div className="voice-settings-panel" role="dialog" aria-label="Voice settings">
      <div className="voice-settings-head">
        <h2>Voice settings</h2>
        <button className="btn" data-variant="ghost" aria-label="Close voice settings" onClick={onClose}>
          Close
        </button>
      </div>
      <div className="voice-settings-body">
        <p className="voice-settings-lede">
          Voice requires the STT and TTS model files (~165–240 MB). They are
          downloaded once into app data; subsequent launches skip this step.
        </p>

        <div className="voice-models-list" data-testid="voice-models-list">
          <ModelRow
            label="STT (Whisper tiny.en, ~74 MB)"
            state={status.stt}
            received={progress.stt.received}
            total={progress.stt.total}
          />
          <ModelRow
            label="TTS (Kokoro q8, ~92 MB)"
            state={status.tts}
            received={progress.tts.received}
            total={progress.tts.total}
          />
        </div>

        {error && <div className="voice-settings-error">{error}</div>}

        {showDownloadButton && (
          <button
            className="btn"
            data-variant={anyFailed ? "danger" : "primary"}
            data-testid="voice-download-btn"
            disabled={downloading}
            onClick={handleDownload}
          >
            {anyFailed ? "Retry download" : "Download models"}
          </button>
        )}

        {allReady && (
          <button
            className="btn"
            data-variant="ghost"
            data-testid="voice-redownload-btn"
            disabled={downloading}
            onClick={handleDownload}
          >
            Re-download
          </button>
        )}

        {allReady && (
          <div className="voice-ptt-test" data-testid="voice-ptt-test">
            <p className="voice-settings-lede">
              Push-to-talk test: enable voice in the top bar and make sure the
              sidecars are running, then hold the button and speak.
            </p>
            <button
              className="btn"
              data-variant={recording ? "danger" : "primary"}
              data-testid="voice-ptt-btn"
              onPointerDown={startPtt}
              onPointerUp={stopPtt}
              onPointerLeave={stopPtt}
            >
              {recording ? "Listening… release to transcribe" : "Hold to talk"}
            </button>
            {pttError && <div className="voice-settings-error">{pttError}</div>}
            {transcript !== null && (
              <div className="voice-ptt-transcript" data-testid="voice-ptt-transcript">
                <strong>Heard:</strong> {transcript || "(nothing)"}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
