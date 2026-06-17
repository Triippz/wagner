import { useEffect, useRef, useState } from "react";
import type { RunSnapshot } from "../../store/types";
import { cmd } from "../bridge";

// Callback type for opening the voice settings panel.
type OpenVoiceSettings = () => void;

type Tone = "drafted" | "running" | "needs-you" | "met" | "halted" | "aborted" | "paused";

function statusTone(run: RunSnapshot | null, needsYou: boolean): { tone: Tone; label: string } {
  if (needsYou) return { tone: "needs-you", label: "Needs you" };
  switch (run?.status) {
    case "running": return { tone: "running", label: "Running" };
    case "met": return { tone: "met", label: "Goal met" };
    case "halted_guardrail": return { tone: "halted", label: "Halted" };
    case "aborted": return { tone: "aborted", label: "Aborted" };
    case "paused": return { tone: "paused", label: "Paused" };
    default: return { tone: "drafted", label: "Idle" };
  }
}

const PHASE_LABEL: Record<string, string> = {
  idle: "idle",
  planning: "planning",
  dispatching: "dispatching",
  judging: "judging",
  blocked: "blocked",
  met: "met",
  halted: "halted",
};

// Voice toggle state machine: off | starting | on | error | models-needed.
// "starting" covers the in-flight period after toggling on before ready=true lands.
// "models-needed" is a transient overlay state: shown briefly to direct the user
// to the settings panel when the backend rejects with "models not ready".
type VoiceState = "off" | "starting" | "on" | "error" | "models-needed";

function voiceLabel(s: VoiceState): string {
  switch (s) {
    case "on":            return "Voice on";
    case "starting":      return "Voice starting";
    case "error":         return "Voice error";
    case "models-needed": return "Download models";
    default:              return "Voice off";
  }
}

function useVoiceToggle(onOpenSettings: OpenVoiceSettings) {
  const [voiceState, setVoiceState] = useState<VoiceState>("off");
  const [toggling, setToggling] = useState(false);
  // Track mount state so async toggle callbacks don't call setState after unmount.
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  // Fetch initial status on mount — best-effort (non-Tauri/mock → stays "off").
  useEffect(() => {
    let alive = true;
    cmd.voiceStatus()
      .then(({ enabled, ready }) => {
        if (!alive) return;
        if (enabled && ready) setVoiceState("on");
        else if (enabled) setVoiceState("starting");
        else setVoiceState("off");
      })
      .catch(() => { if (alive) setVoiceState("off"); });
    return () => { alive = false; };
  }, []);

  const toggle = () => {
    if (toggling) return;
    // When in models-needed state, clicking the button opens the settings panel.
    if (voiceState === "models-needed") {
      onOpenSettings();
      return;
    }
    const turningOn = voiceState !== "on";
    setToggling(true);
    setVoiceState(turningOn ? "starting" : "off");
    cmd.voiceSetEnabled(turningOn)
      .then(({ enabled, ready }) => {
        if (!mountedRef.current) return;
        if (enabled && ready) setVoiceState("on");
        else if (enabled) setVoiceState("starting");
        else setVoiceState("off");
      })
      .catch((e: unknown) => {
        if (!mountedRef.current) return;
        const msg = String(e);
        if (turningOn && msg.includes("models not ready")) {
          // Surface the prompt directing the user to the settings panel.
          setVoiceState("models-needed");
          onOpenSettings();
        } else {
          setVoiceState(turningOn ? "error" : "off");
        }
      })
      .finally(() => { if (mountedRef.current) setToggling(false); });
  };

  return { voiceState, toggling, toggle };
}

interface Props {
  run: RunSnapshot | null;
  needsYou: boolean;
  busy: boolean;
  onAbort: () => void;
  onNewRun: () => void;
  onOpenVoiceSettings: () => void;
}

export function TopBar({ run, needsYou, busy, onAbort, onNewRun, onOpenVoiceSettings }: Props) {
  const { voiceState, toggling, toggle } = useVoiceToggle(onOpenVoiceSettings);
  const { tone, label } = statusTone(run, needsYou);
  const cost = run?.guardrails.cost;
  const used = cost?.used ?? 0;
  const budget = cost?.budget ?? null;
  const pct = budget && budget > 0 ? Math.min(100, (used / budget) * 100) : null;
  const iter = run?.iteration ?? 0;
  const maxIter = run?.guardrails.max_iterations ?? null;

  return (
    <header className="topbar">
      <div className="brand">
        <b>Wagner</b>
        <span>Edge</span>
      </div>

      <div className="topbar-goal" title={run?.goal}>
        {run?.goal ?? "No active run"}
      </div>

      <div className="topbar-meta">
        <span className="pill" data-tone={tone}>
          <span className="dot" />
          {label}
        </span>

        {run && (
          <>
            <div className="meta-cell">
              <span className="k">Phase</span>
              <span className="v">{PHASE_LABEL[run.phase ?? "idle"] ?? run.phase}</span>
            </div>
            <div className="meta-cell">
              <span className="k">Iter</span>
              <span className="v">{maxIter ? `${iter}/${maxIter}` : iter}</span>
            </div>
            <div className="meta-cell">
              <span className="k">Spend</span>
              <span className="v">${used.toFixed(2)}</span>
              {pct !== null && (
                <span className="cost-meter" data-over={pct >= 100}>
                  <i style={{ width: `${pct}%` }} />
                </span>
              )}
            </div>
          </>
        )}

        <button
          className="btn voice-toggle"
          data-variant="ghost"
          data-voice={voiceState}
          aria-label={voiceLabel(voiceState)}
          disabled={toggling}
          onClick={toggle}
        >
          {voiceLabel(voiceState)}
        </button>
        <button
          className="btn voice-settings-btn"
          data-variant="ghost"
          aria-label="Open voice settings"
          onClick={onOpenVoiceSettings}
          title="Voice settings"
        >
          Settings
        </button>

        {busy ? (
          <button className="btn" data-variant="danger" onClick={onAbort}>
            Abort
          </button>
        ) : (
          run && (
            <button className="btn" data-variant="ghost" onClick={onNewRun}>
              New run
            </button>
          )
        )}
      </div>
    </header>
  );
}
