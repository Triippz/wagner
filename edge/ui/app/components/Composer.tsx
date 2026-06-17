import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { cmd, type CliStatus } from "../bridge";

interface Props {
  onLaunched: () => void;
}

// New-session entry screen: pick a folder (native dialog) + name the first goal.
// Guardrails (max-iterations / cost / blocked-timeout) and the test command are
// gone — the target repo's own CLAUDE.md / AGENTS.md declares how it tests, and
// the host applies guardrail defaults when omitted.
export function Composer({ onLaunched }: Props) {
  const [goal, setGoal] = useState("");
  const [projectDir, setProjectDir] = useState("");
  const [preflight, setPreflight] = useState<CliStatus | null>(null);
  const [launching, setLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    cmd.preflight().then(setPreflight).catch(() => setPreflight(null));
  }, []);

  const canLaunch =
    goal.trim().length > 0 && projectDir.trim().length > 0 && !launching;

  async function chooseFolder() {
    setError(null);
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: "Choose a project folder",
      });
      if (typeof picked === "string") setProjectDir(picked);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function launch() {
    setError(null);
    setLaunching(true);
    try {
      const ok = await cmd.validateProjectDir(projectDir.trim());
      if (!ok)
        throw new Error(`project directory does not exist: ${projectDir.trim()}`);
      // No guardrails: the host applies R-GUARDRAILS defaults.
      await cmd.startRun({ goal: goal.trim(), projectDir: projectDir.trim(), docs: [] });
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
          <h1>New session</h1>
          <p className="lede">
            Pick a project folder and name the first goal. The oracle plans, the
            roster executes, and the session keeps going after you close the
            window.
          </p>
        </div>

        <div className="field">
          <label htmlFor="goal">First goal</label>
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
          <label>Project folder</label>
          <div className="row">
            <button
              className="btn"
              data-variant="ghost"
              type="button"
              onClick={chooseFolder}
            >
              Choose folder…
            </button>
            <span className="path" title={projectDir}>
              {projectDir || "No folder chosen"}
            </span>
          </div>
          <span className="hint">
            Where the agents run — its .claude / AGENTS.md / MCP config and test
            setup apply here.
          </span>
        </div>

        {error && <p className="err-line">{error}</p>}

        <div className="composer-foot">
          <div className="preflight">
            {preflight ? (
              <>
                <span className={preflight.claude ? "ok" : "no"}>
                  claude {preflight.claude ? "✓" : "—"}
                </span>
                <span className={preflight.codex ? "ok" : "no"}>
                  codex {preflight.codex ? "✓" : "—"}
                </span>
              </>
            ) : (
              <span className="no">checking engines…</span>
            )}
          </div>
          <button
            className="btn"
            data-variant="primary"
            disabled={!canLaunch}
            onClick={launch}
          >
            {launching ? "Launching…" : "Start session"}
          </button>
        </div>
      </div>
    </div>
  );
}
