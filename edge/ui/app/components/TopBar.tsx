import { useEffect, useState } from "react";
import type { RunSnapshot } from "../../store/types";
import { cmd } from "../bridge";

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

// Voice toggle state machine: off | starting | on | error.
// "starting" covers the in-flight period after toggling on before ready=true lands.
type VoiceState = "off" | "starting" | "on" | "error";

function voiceLabel(s: VoiceState): string {
  switch (s) {
    case "on":       return "Voice on";
    case "starting": return "Voice starting";
    case "error":    return "Voice error";
    default:         return "Voice off";
  }
}

function useVoiceToggle() {
  const [voiceState, setVoiceState] = useState<VoiceState>("off");
  const [toggling, setToggling] = useState(false);

  // Fetch initial status on mount — best-effort (non-Tauri/mock → stays "off").
  useEffect(() => {
    cmd.voiceStatus()
      .then(({ enabled, ready }) => {
        if (enabled && ready) setVoiceState("on");
        else if (enabled) setVoiceState("starting");
        else setVoiceState("off");
      })
      .catch(() => setVoiceState("off"));
  }, []);

  const toggle = () => {
    if (toggling) return;
    const turningOn = voiceState !== "on";
    setToggling(true);
    setVoiceState(turningOn ? "starting" : "off");
    cmd.voiceSetEnabled(turningOn)
      .then(({ enabled, ready }) => {
        if (enabled && ready) setVoiceState("on");
        else if (enabled) setVoiceState("starting");
        else setVoiceState("off");
      })
      .catch(() => setVoiceState(turningOn ? "error" : "off"))
      .finally(() => setToggling(false));
  };

  return { voiceState, toggling, toggle };
}

interface Props {
  run: RunSnapshot | null;
  needsYou: boolean;
  busy: boolean;
  onAbort: () => void;
  onNewRun: () => void;
}

export function TopBar({ run, needsYou, busy, onAbort, onNewRun }: Props) {
  const { voiceState, toggling, toggle } = useVoiceToggle();
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
