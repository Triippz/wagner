import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { Surface } from "../surfaces/surface";
import { activeRun, openTransmission, sessionRows } from "../store/reducer";
import { cmd, type RunSummary } from "./bridge";
import { TopBar } from "./components/TopBar";
import { OperativeRail } from "./components/OperativeRail";
import { SessionRail } from "./components/SessionRail";
import { Inspector } from "./components/Inspector";
import { TransmissionPrompt } from "./components/TransmissionPrompt";
import { Composer } from "./components/Composer";
import { VaultPanel } from "./components/VaultPanel";

type ActiveView = "console" | "vault";

export function App({ surface }: { surface: Surface }) {
  const state = useSyncExternalStore(
    useCallback((cb: () => void) => surface.onChange(cb), [surface]),
    useCallback(() => surface.getState(), [surface]),
  );

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [answeredIds, setAnsweredIds] = useState<ReadonlySet<string>>(new Set());
  const [pendingAnswer, setPendingAnswer] = useState<string | null>(null);
  const [summaries, setSummaries] = useState<RunSummary[]>([]);
  const [activeView, setActiveView] = useState<ActiveView>("console");

  // Load persisted sessions on boot so the rail isn't empty before live events
  // (best-effort: in a non-Tauri/mock browser this invoke rejects → no summaries).
  useEffect(() => {
    cmd.listRuns().then(setSummaries).catch(() => setSummaries([]));
  }, []);

  const run = activeRun(state);
  const sessions = useMemo(
    () => sessionRows(state.runs, summaries),
    [state.runs, summaries]
  );
  const operatives = useMemo(() => Object.values(state.operatives), [state.operatives]);
  const rawOpen = openTransmission(state);
  const open = rawOpen && !answeredIds.has(rawOpen.id) ? rawOpen : null;
  const needsYou = open !== null;
  const selected = selectedId ? state.operatives[selectedId] ?? null : null;
  const busy = run?.status === "running";

  // Focus a session from the rail; reopen it on the host if it isn't live.
  const onSelectSession = useCallback(
    (runId: string) => {
      surface.selectRun(runId);
      setSelectedId(null);
      const live = state.runs[runId];
      if (!live || live.status !== "running") {
        cmd.resumeRun(runId).catch((e) => console.error("[wagner] resume failed:", e));
      }
    },
    [surface, state.runs]
  );

  const onAnswer = useCallback((id: string, response: string) => {
    setPendingAnswer(id);
    cmd
      .answerTransmission(id, response)
      .catch((e) => console.error("[wagner] answer failed:", e))
      .finally(() => {
        setAnsweredIds((s) => new Set(s).add(id));
        setPendingAnswer(null);
      });
  }, []);

  const newRun = useCallback(() => {
    setComposing(true);
    setSelectedId(null);
  }, []);

  // Composer shows when explicitly composing or when there are no sessions at
  // all. Once any session exists (live or persisted), the console + rail show —
  // even before one is focused, so a closed session can be reopened from the rail.
  if (composing || sessions.length === 0) {
    return (
      <div className="app">
        <TopBar run={run} needsYou={needsYou} busy={false} onAbort={() => {}} onNewRun={newRun} />
        <Composer onLaunched={() => setComposing(false)} />
      </div>
    );
  }

  return (
    <div className="app">
      <TopBar run={run} needsYou={needsYou} busy={!!busy} onAbort={() => cmd.abort()} onNewRun={newRun} />
      <div className="view-body">
        <nav className="view-rail">
          <button
            className={`view-tab${activeView === "console" ? " active" : ""}`}
            onClick={() => setActiveView("console")}
          >
            Console
          </button>
          <button
            className={`view-tab${activeView === "vault" ? " active" : ""}`}
            onClick={() => setActiveView("vault")}
            disabled={needsYou}
            title={needsYou ? "Answer the pending request first" : undefined}
          >
            Vault
          </button>
        </nav>
        {activeView === "vault" ? (
          <VaultPanel surface={surface} projectDir="" />
        ) : (
          <div className="console">
            <SessionRail
              sessions={sessions}
              selectedRunId={state.selectedRunId}
              onSelect={onSelectSession}
              onNewSession={newRun}
            />
            <OperativeRail operatives={operatives} selectedId={selectedId} onSelect={setSelectedId} />
            <main className="main">
              {open && (
                <TransmissionPrompt
                  transmission={open}
                  pending={pendingAnswer === open.id}
                  onAnswer={onAnswer}
                />
              )}
              <Inspector operative={selected} run={run} />
            </main>
          </div>
        )}
      </div>
    </div>
  );
}
