import { useCallback, useMemo, useState, useSyncExternalStore } from "react";
import type { Surface } from "../surfaces/surface";
import { activeRun, openTransmission } from "../store/reducer";
import { cmd } from "./bridge";
import { TopBar } from "./components/TopBar";
import { OperativeRail } from "./components/OperativeRail";
import { Inspector } from "./components/Inspector";
import { TransmissionPrompt } from "./components/TransmissionPrompt";
import { Composer } from "./components/Composer";

export function App({ surface }: { surface: Surface }) {
  const state = useSyncExternalStore(
    useCallback((cb: () => void) => surface.onChange(cb), [surface]),
    useCallback(() => surface.getState(), [surface]),
  );

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [answeredIds, setAnsweredIds] = useState<ReadonlySet<string>>(new Set());
  const [pendingAnswer, setPendingAnswer] = useState<string | null>(null);

  const run = activeRun(state);
  const operatives = useMemo(() => Object.values(state.operatives), [state.operatives]);
  const rawOpen = openTransmission(state);
  const open = rawOpen && !answeredIds.has(rawOpen.id) ? rawOpen : null;
  const needsYou = open !== null;
  const selected = selectedId ? state.operatives[selectedId] ?? null : null;
  const busy = run?.status === "running";

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

  if (!run || composing) {
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
      <div className="console">
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
    </div>
  );
}
