import { useEffect, useState } from "react";
import { cmd, type CliStatus } from "../bridge";

interface Props {
  onLaunched: () => void;
}

export function Composer({ onLaunched }: Props) {
  const [goal, setGoal] = useState("");
  const [projectDir, setProjectDir] = useState("");
  const [maxIter, setMaxIter] = useState("");
  const [costBudget, setCostBudget] = useState("");
  const [blockedTimeout, setBlockedTimeout] = useState("120");
  const [suite, setSuite] = useState("");
  const [preflight, setPreflight] = useState<CliStatus | null>(null);
  const [launching, setLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    cmd.preflight().then(setPreflight).catch(() => setPreflight(null));
  }, []);

  const canLaunch = goal.trim().length > 0 && !launching;

  async function launch() {
    setError(null);
    setLaunching(true);
    try {
      if (projectDir.trim()) {
        const ok = await cmd.validateProjectDir(projectDir.trim());
        if (!ok) throw new Error(`project directory does not exist: ${projectDir.trim()}`);
      }
      await cmd.startRun({
        goal: goal.trim(),
        projectDir: projectDir.trim(),
        docs: [],
        guardrails: {
          max_iterations: maxIter.trim() ? Number(maxIter) : null,
          blocked_timeout_secs: Number(blockedTimeout) || 120,
          cost_budget: costBudget.trim() ? Number(costBudget) : null,
          suite_command: suite.trim() || null,
        },
      });
      onLaunched();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setLaunching(false);
    }
  }

  return (
    <div className="center">
      <div className="composer">
        <div>
          <h1>Launch a run</h1>
          <p className="lede">
            Give the org a goal and a working directory. The oracle plans, the
            roster executes, and the run keeps going after you close the window.
          </p>
        </div>

        <div className="field">
          <label htmlFor="goal">Goal</label>
          <textarea
            id="goal"
            className="textarea"
            placeholder="e.g. Add a /healthz endpoint with a test and wire it into the router."
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
            autoFocus
          />
        </div>

        <div className="field">
          <label htmlFor="dir">Project directory</label>
          <input
            id="dir"
            className="input"
            placeholder="~/code/my-project   (blank = this app's working dir)"
            value={projectDir}
            onChange={(e) => setProjectDir(e.target.value)}
            spellCheck={false}
          />
          <span className="hint">Where the agents run — their per-project .claude / AGENTS.md / MCP config applies here.</span>
        </div>

        <div className="row">
          <div className="field">
            <label htmlFor="iter">Max iterations</label>
            <input id="iter" className="input" inputMode="numeric" placeholder="uncapped"
              value={maxIter} onChange={(e) => setMaxIter(e.target.value)} />
          </div>
          <div className="field">
            <label htmlFor="cost">Cost budget (USD)</label>
            <input id="cost" className="input" inputMode="decimal" placeholder="none"
              value={costBudget} onChange={(e) => setCostBudget(e.target.value)} />
          </div>
        </div>

        <div className="row">
          <div className="field">
            <label htmlFor="bt">Blocked timeout (s)</label>
            <input id="bt" className="input" inputMode="numeric"
              value={blockedTimeout} onChange={(e) => setBlockedTimeout(e.target.value)} />
          </div>
          <div className="field">
            <label htmlFor="suite">Test command</label>
            <input id="suite" className="input" placeholder="e.g. cargo test" spellCheck={false}
              value={suite} onChange={(e) => setSuite(e.target.value)} />
          </div>
        </div>

        {error && <p className="err-line">{error}</p>}

        <div className="composer-foot">
          <div className="preflight">
            {preflight ? (
              <>
                <span className={preflight.claude ? "ok" : "no"}>claude {preflight.claude ? "✓" : "—"}</span>
                <span className={preflight.codex ? "ok" : "no"}>codex {preflight.codex ? "✓" : "—"}</span>
              </>
            ) : (
              <span className="no">checking engines…</span>
            )}
          </div>
          <button className="btn" data-variant="primary" disabled={!canLaunch} onClick={launch}>
            {launching ? "Launching…" : "Launch run"}
          </button>
        </div>
      </div>
    </div>
  );
}
