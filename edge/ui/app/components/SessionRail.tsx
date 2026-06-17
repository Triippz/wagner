import type { SessionRow } from "../../store/reducer";

interface Props {
  sessions: SessionRow[];
  selectedRunId: string | null;
  /** Focus a session; closed sessions are resumed by the parent. */
  onSelect: (runId: string) => void;
  onNewSession: () => void;
}

// Left rail listing every session (live + persisted) with a status dot. Clicking
// a row focuses it; a closed session is reopened by the parent (resume_run).
export function SessionRail({
  sessions,
  selectedRunId,
  onSelect,
  onNewSession,
}: Props) {
  return (
    <aside className="session-rail">
      <div className="session-rail-head">
        <span>Sessions</span>
        <button
          className="btn"
          data-variant="ghost"
          type="button"
          onClick={onNewSession}
          title="New session"
        >
          + New
        </button>
      </div>
      <ul className="session-list">
        {sessions.map((s) => (
          <li key={s.run_id}>
            <button
              type="button"
              className={
                "session-row" + (s.run_id === selectedRunId ? " is-selected" : "")
              }
              onClick={() => onSelect(s.run_id)}
            >
              <span className={"dot dot-" + s.dot} aria-label={s.dot} />
              <span className="session-name">{s.name}</span>
              <span className="session-goal">{s.goal}</span>
            </button>
          </li>
        ))}
        {sessions.length === 0 && (
          <li className="session-empty">No sessions yet</li>
        )}
      </ul>
    </aside>
  );
}
