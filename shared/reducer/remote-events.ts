// Remote session event kinds (wedge-002, T011). Appended to the carried
// append-only run-event log and folded by `remote-reducer.ts` (Article VIII).
//
// CRITICAL (F-1 / SC-006): these events record that a remote action HAPPENED
// plus its metadata (who, which command argv, which path) — NEVER the
// run-bearing OUTPUT (stdout/stderr/file bytes/diff). Output streams over the
// P2P channel to the operator's device and is never persisted or synced. The
// types below deliberately have no output/content field; the schema
// (`remote-event.schema.json`, additionalProperties:false) enforces the same
// structurally.

/** Why a remote session ended for a client. */
export type DetachReason = "closed" | "dropped";

/** Run-control ① intent (mirrors remote-control.schema.json `kind`). */
export type ControlKind = "steer" | "answer_permission" | "run_skill";

/** Why a dev-context file read was refused (repo-scope default-deny). */
export type RefusedReason = "out_of_scope";

interface RemoteEventBase {
  /** ULID of this event. */
  event_id: string;
  /** ULID of the owning run. */
  run_id: string;
  /** ISO-8601 (`…Z`). */
  ts: string;
}

/** One immutable entry in the remote portion of a run's log. Discriminated on `type`. */
export type RemoteEvent =
  | (RemoteEventBase & { type: "remote.armed"; operator_id: string; node_id: string; ticket_id: string })
  | (RemoteEventBase & { type: "remote.disarmed" })
  | (RemoteEventBase & { type: "remote.attached"; operator_id: string; client_id: string })
  | (RemoteEventBase & { type: "remote.detached"; client_id: string; reason: DetachReason })
  | (RemoteEventBase & { type: "remote.control"; client_id: string; kind: ControlKind; ref: string })
  | (RemoteEventBase & { type: "dev_context.command"; client_id: string; argv: string[]; cwd: string })
  | (RemoteEventBase & { type: "dev_context.file_read"; client_id: string; path: string; bytes: number })
  | (RemoteEventBase & { type: "dev_context.tree"; client_id: string; root: string })
  | (RemoteEventBase & { type: "dev_context.refused"; client_id: string; path: string; reason: RefusedReason });

export type RemoteEventType = RemoteEvent["type"];
