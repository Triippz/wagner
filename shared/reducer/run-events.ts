// Event-sourced run spine (audit F1, 2026-06-15).
//
// The carried edge persists run state as an atomic snapshot (`state/store.rs`) and
// emits `WagnerEvent`s only as a transient UI projection. Article VIII / FR-003 /
// SC-005 require the run's *synced metadata* to be a projection of an append-only
// log. This module declares that log's events; `run-reducer.ts` folds them. The
// ported orchestrator (run_loop, T024) emits these and atomically appends them so
// the log — not the snapshot — is the source of truth.

/** Status as produced by the run-event log. ("drafted" is a pre-execution UI state,
 *  never an event-log status — the log begins at `run.created` = running.) */
export type RunStatus = "running" | "paused" | "met" | "halted_guardrail" | "aborted";

/** A run that has reached a terminal state. */
export type TerminalStatus = "met" | "halted_guardrail" | "aborted";

/** Why a run halted on a guardrail (carried `run-state.schema.json` halt_reason). */
export type HaltReason = "iterations" | "cost" | "blocked_timeout";

interface RunEventBase {
  schema: "wagner-run-event.v1";
  /** ULID of this event. */
  event_id: string;
  /** ULID of the owning run (idempotency key for the synced snapshot). */
  run_id: string;
  /** ISO-8601 (`…Z`, no sub-seconds — matches the carried atomic-write convention). */
  ts: string;
}

/** One immutable entry in a run's append-only run-event log. Discriminated on `type`. */
export type RunEvent =
  | (RunEventBase & { type: "created"; goal: string; started_at: string })
  | (RunEventBase & { type: "iteration_advanced" })
  | (RunEventBase & { type: "cost_folded"; delta: number })
  | (RunEventBase & { type: "status_changed"; status: RunStatus })
  | (RunEventBase & {
      type: "finished";
      status: TerminalStatus;
      halt_reason: HaltReason | null;
      ended_at: string;
    });

export type RunEventType = RunEvent["type"];
