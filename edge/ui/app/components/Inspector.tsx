import type { Operative, RunSnapshot, Subtask } from "../../store/types";

function clock(ts: string): string {
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return "--:--:--";
  return d.toLocaleTimeString("en-GB", { hour12: false });
}

function OperativeView({ op }: { op: Operative }) {
  const lines = [...op.transcript].reverse();
  return (
    <>
      <div className="panel-head">
        <div>
          <h2>{op.name}</h2>
          <span className="inspector-id">
            {op.faction} · {op.district} · {op.activity}
          </span>
        </div>
        <span className="pill" data-tone={op.state === "blocked" ? "needs-you" : "running"}>
          <span className="dot" />
          {op.state}
        </span>
      </div>
      <div className="panel-body">
        {lines.length === 0 ? (
          <p className="empty-note" style={{ marginTop: 40 }}>
            No transcript yet — {op.name} hasn't reported in.
          </p>
        ) : (
          <div className="transcript">
            {lines.map((l, i) => (
              <div className="tline" key={`${l.ts}-${i}`}>
                <span className="t-ts">{clock(l.ts)}</span>
                <span className="t-act">{l.activity}</span>
                <span className="t-msg">{l.message}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  );
}

function SubtaskRow({ st }: { st: Subtask }) {
  return (
    <div className="subtask">
      <span className="st-mark" data-s={st.state} />
      <span className="st-body">
        <div className="st-prompt" title={st.prompt}>{st.prompt}</div>
        <div className="st-agent">→ {st.agent_id}{st.result_summary ? ` · ${st.result_summary}` : ""}</div>
      </span>
      <span className="op-state" data-s={st.state}>{st.state}</span>
    </div>
  );
}

function RunView({ run }: { run: RunSnapshot | null }) {
  const subtasks = run?.subtasks ?? [];
  return (
    <>
      <div className="panel-head">
        <div>
          <h2>Run overview</h2>
          <span className="inspector-id">{run ? run.run_id : "no run"}</span>
        </div>
      </div>
      <div className="panel-body">
        {!run && <p className="empty-note" style={{ marginTop: 40 }}>No run in progress.</p>}
        {run && (
          <>
            <p className="sectionlabel">Goal</p>
            <p style={{ margin: "4px 0 0", maxWidth: "72ch", lineHeight: 1.55 }}>{run.goal}</p>
            <p className="sectionlabel">Dispatched subtasks</p>
            {subtasks.length === 0 ? (
              <p className="empty-note" style={{ marginTop: 16, alignItems: "flex-start" }}>
                Nothing dispatched yet — the oracle is still planning.
              </p>
            ) : (
              <div className="subtasks">
                {subtasks.map((st) => <SubtaskRow key={st.id} st={st} />)}
              </div>
            )}
          </>
        )}
      </div>
    </>
  );
}

interface Props {
  operative: Operative | null;
  run: RunSnapshot | null;
}

export function Inspector({ operative, run }: Props) {
  return operative ? <OperativeView op={operative} /> : <RunView run={run} />;
}
