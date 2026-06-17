// Live Tauri bridge + typed command helpers.
//
// `tauriBridge` is the concrete `TauriBridge` the IPC transport folds the host
// event stream over. The command helpers wrap `invoke` for the control path
// (launch / steer / abort / answer) — called directly rather than through
// `surface.send`, since the desktop path is in-process IPC. Tauri v2 converts
// camelCase arg keys to the Rust snake_case params; nested objects (guardrails)
// pass through serde verbatim, so their keys stay snake_case.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TauriBridge } from "../transport/ipc";
import type { RunSnapshot, RunStatus } from "../store/types";

/** Lightweight session summary for the rail (mirrors host `RunSummary`). */
export interface RunSummary {
  run_id: string;
  name: string;
  project_dir: string;
  status: RunStatus;
  updated_at: string;
  goal: string;
}

export const tauriBridge: TauriBridge = {
  listen: (channel, handler) => listen(channel, (e) => handler(e.payload)),
  invoke: (command, args) => invoke(command, args),
};

/** CLI/host preflight: which engines are available, whether keys are present. */
export interface CliStatus {
  claude: boolean;
  codex: boolean;
  has_api_key: boolean;
  [k: string]: unknown;
}

export interface AgentIdentity {
  id: string;
  name: string;
  role?: string;
  [k: string]: unknown;
}

export interface GuardrailInput {
  max_iterations: number | null;
  blocked_timeout_secs: number;
  cost_budget: number | null;
  suite_command: string | null;
}

export interface StartRunInput {
  goal: string;
  projectDir: string;
  docs: string[];
  guardrails: GuardrailInput;
}

export const cmd = {
  preflight: () => invoke<CliStatus>("preflight"),
  agentCatalog: (projectDir: string) =>
    invoke<AgentIdentity[]>("agent_catalog", { projectDir }),
  validateProjectDir: (projectDir: string) =>
    invoke<boolean>("validate_project_dir", { projectDir }),
  startRun: (input: StartRunInput) =>
    invoke<string>("start_run", {
      goal: input.goal,
      docs: input.docs,
      guardrails: input.guardrails,
      projectDir: input.projectDir,
      roster: null,
    }),
  steer: (text: string) => invoke<void>("steer", { text }),
  abort: () => invoke<void>("abort"),
  /** Persisted sessions for the rail, newest-first. */
  listRuns: () => invoke<RunSummary[]>("list_runs"),
  /** Full state of one persisted session (reopening from the rail). */
  getRun: (runId: string) => invoke<RunSnapshot>("get_run", { runId }),
  /** Resume a closed/paused session — rebuilds the loop and re-enters it. */
  resumeRun: (runId: string) => invoke<void>("resume_run", { runId }),
  answerTransmission: (id: string, response: string) =>
    invoke<void>("answer_transmission", { id, response }),
};
