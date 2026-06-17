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

/** A tiered vault-retrieval hit (mirrors host `TieredResultDto`). */
export interface TieredResultDto {
  uid: string;
  summary: string;
  snippet: string;
  tier_kind: "summary" | "section" | "full" | "related";
}

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
  /** Optional: the redesigned entry screen omits these (host applies defaults). */
  guardrails?: GuardrailInput;
}

export interface VaultNodeDto {
  uid: string;
  title: string;
  tier: string;
  lifecycle: string;
}

export interface VaultEdgeDto {
  sourceUid: string;
  targetUid: string;
  relType: string;
}

export interface VaultGraphDto {
  nodes: VaultNodeDto[];
  edges: VaultEdgeDto[];
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
      guardrails: input.guardrails ?? null,
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
  /** Add a goal to a session: injected live, or persisted + resumed if closed. */
  addGoal: (runId: string, text: string) =>
    invoke<void>("add_goal", { runId, text }),
  /** Tiered vault retrieval for a project (summary→section→full→related). */
  vaultSummary: (projectDir: string, query: string) =>
    invoke<TieredResultDto[]>("vault_summary", { projectDir, query }),
  /** Promote a staged vault note to the curated dir. */
  approveStaging: (projectDir: string, uid: string) =>
    invoke<void>("approve_staging", { projectDir, uid }),
  /** Uids of vault notes awaiting approval. */
  listStaging: (projectDir: string) =>
    invoke<string[]>("list_staging", { projectDir }),
  answerTransmission: (id: string, response: string) =>
    invoke<void>("answer_transmission", { id, response }),
  vaultGraph: (projectDir: string) =>
    invoke<VaultGraphDto>("vault_graph", { projectDir }),
  /** Current voice-engine state. */
  voiceStatus: () => invoke<{ enabled: boolean; ready: boolean }>("voice_status"),
  /** Enable or disable the voice engine; returns updated state. */
  voiceSetEnabled: (on: boolean) =>
    invoke<{ enabled: boolean; ready: boolean }>("voice_set_enabled", { on }),
  /** Per-model download/readiness state for STT and TTS. */
  voiceModelsStatus: () =>
    invoke<{ stt: string; tts: string }>("voice_models_status"),
  /** Trigger download of all missing voice models into app-data. */
  voiceDownloadModels: () => invoke<void>("voice_download_models"),
};
