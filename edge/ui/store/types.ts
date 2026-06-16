// Frontend mirrors of the host's schema-validated payloads (wagner-event.v1,
// wagner-run.v1, transmission.v1). Kept structurally in lockstep with
// apps/wagner/schemas/*.json.

import type { Activity, District } from "../world/districts";

export type Faction = "architects" | "forgers";
export type OperativeState = "idle" | "thinking" | "working" | "blocked";

export interface WagnerEvent {
  schema: "wagner-event.v1";
  event_id: string;
  run_id: string;
  operative_id: string;
  /** Display name of the hired agent (floor label); falls back to the id. */
  operative_name?: string;
  faction: Faction;
  activity: Activity;
  district: District;
  state: OperativeState;
  message?: string;
  handoff_target_operative_id: string | null;
  ts: string;
}

export type RunStatus =
  | "drafted"
  | "running"
  | "met"
  | "halted_guardrail"
  | "aborted"
  | "paused";

/** Current loop step — what's happening right now (mission bar). */
export type RunPhase =
  | "idle"
  | "planning"
  | "dispatching"
  | "judging"
  | "blocked"
  | "met"
  | "halted";

export type SubtaskState = "queued" | "running" | "done" | "failed";

/** One dispatched unit of work, as the inspector shows it. */
export interface Subtask {
  id: string;
  /** id of the hired-roster agent this subtask was dispatched to. */
  agent_id: string;
  assignment_rationale?: string;
  prompt: string;
  state: SubtaskState;
  worktree?: string | null;
  result_summary?: string | null;
  parent_event_ids?: string[];
}

export interface RunSnapshot {
  schema: "wagner-run.v1";
  run_id: string;
  goal: string;
  status: RunStatus;
  /** Current loop step (omitted by older run-states; defaults to "idle"). */
  phase?: RunPhase;
  iteration: number;
  guardrails: {
    max_iterations?: number | null;
    iterations_used?: number;
    blocked_timeout_secs: number;
    cost: { mode: "cli_usage" | "wallclock"; budget?: number | null; used?: number };
  };
  halt_reason?: "iterations" | "cost" | "blocked_timeout" | null;
  /** Dispatched subtasks (the inspector's "current objective" per agent). */
  subtasks?: Subtask[];
}

export interface TransmissionOption {
  id: string;
  label: string;
}

export interface Transmission {
  schema: "transmission.v1";
  id: string;
  subtask_id: string;
  kind: "permission" | "question";
  prompt: string;
  options: TransmissionOption[];
  raised_at: string;
  answered_at?: string | null;
  response?: string | null;
  state: "open" | "answered" | "timed_out";
}

/** One line of an operative's thinking transcript (inspector). */
export interface TranscriptEntry {
  ts: string;
  activity: Activity;
  message: string;
}

/** A live operative on the floor, derived from the event stream. */
export interface Operative {
  id: string;
  /** Hired-agent display name (floor label). */
  name: string;
  faction: Faction;
  district: District;
  /** The verb the operative is currently performing (drives its work vignette). */
  activity: Activity;
  state: OperativeState;
  bubble?: string;
  handoffTo: string | null;
  lastTs: string;
  /** Accumulated thinking transcript — one entry per message-bearing event. */
  transcript: TranscriptEntry[];
}
