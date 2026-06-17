import { useEffect, useState } from "react";
import { cmd } from "../bridge";

// Model download states, mirroring the Rust lane contract.
type ModelState = "absent" | "downloading" | "verifying" | "ready" | "failed";

interface ModelStatus {
  stt: ModelState;
  tts: ModelState;
}

// Progress payload from the `wagner://voice-download` channel.
interface DownloadProgressPayload {
  model: "stt" | "tts";
  state: ModelState;
  received: number;
  total: number;
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

  // Fetch initial status on mount — best-effort (non-Tauri/mock → stays absent).
  useEffect(() => {
    cmd.voiceModelsStatus()
      .then((s) => setStatus({ stt: s.stt as ModelState, tts: s.tts as ModelState }))
      .catch(() => {}); // non-Tauri environment: leave default "absent"
  }, []);

  // Subscribe to live download progress via the mock/side-channel `on` API,
  // mirroring how VaultPanel uses the side-channel for vault_graph_result.
  useEffect(() => {
    if (useMock) {
      const handle = (window as unknown as { __wagner?: MockHandle }).__wagner;
      if (!handle?.on) return;
      return handle.on("wagner://voice-download", (payload) => {
        const p = payload as DownloadProgressPayload;
        setStatus((prev) => ({ ...prev, [p.model]: p.state }));
        setProgress((prev) => ({
          ...prev,
          [p.model]: { received: p.received, total: p.total },
        }));
      });
    }
    // In native mode the Rust lane emits progress over the same channel name;
    // we'd wire a real listen() here in a future step — for now the mock seam
    // is the primary test path for the UI.
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
      </div>
    </div>
  );
}
