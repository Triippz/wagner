// Pure run reducer (audit F1) — folds an append-only run-event log into the run
// metadata snapshot the wedge syncs (FR-006). No I/O, no UI dependency (Article VIII).
// Replaying a log from empty reproduces the live snapshot (FR-003, SC-005).

import type { RunEvent, RunStatus, HaltReason } from "./run-events";

/** The run metadata projected from the event log and synced to the hub (FR-006).
 *  `operator_id` and `project_key` are added at the sync boundary (identity +
 *  enrollment), not folded here — the log is run-local. */
export interface RunMetadataSnapshot {
  run_id: string;
  goal: string;
  status: RunStatus;
  halt_reason: HaltReason | null;
  iterations_used: number;
  cost_used: number;
  started_at: string;
  ended_at: string | null;
}

/** Fold one event into the snapshot. Pure: returns a new snapshot, never mutates.
 *  `run.created` seeds from null; any other event before `created` is a malformed log. */
export function foldRunEvent(
  snapshot: RunMetadataSnapshot | null,
  event: RunEvent,
): RunMetadataSnapshot {
  if (event.type === "created") {
    return {
      run_id: event.run_id,
      goal: event.goal,
      status: "running",
      halt_reason: null,
      iterations_used: 0,
      cost_used: 0,
      started_at: event.started_at,
      ended_at: null,
    };
  }

  if (snapshot === null) {
    throw new Error(
      `run-event '${event.type}' (${event.event_id}) arrived before run.created — malformed log`,
    );
  }

  switch (event.type) {
    case "iteration_advanced":
      return { ...snapshot, iterations_used: snapshot.iterations_used + 1 };
    case "cost_folded":
      return { ...snapshot, cost_used: snapshot.cost_used + event.delta };
    case "status_changed":
      return { ...snapshot, status: event.status };
    case "finished":
      return {
        ...snapshot,
        status: event.status,
        halt_reason: event.halt_reason,
        ended_at: event.ended_at,
      };
    default: {
      const unreachable: never = event;
      throw new Error(`unhandled run-event: ${JSON.stringify(unreachable)}`);
    }
  }
}

/** Replay a run's full event log from empty into its metadata snapshot. */
export function replayRun(events: readonly RunEvent[]): RunMetadataSnapshot {
  let snapshot: RunMetadataSnapshot | null = null;
  for (const event of events) snapshot = foldRunEvent(snapshot, event);
  if (snapshot === null) throw new Error("empty run-event log — no run.created");
  return snapshot;
}
