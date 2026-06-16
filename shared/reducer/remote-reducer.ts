// Pure remote-session reducer (wedge-002, T011). Folds the remote portion of
// the append-only log into the session projection the surface renders. No I/O,
// no UI dependency (Article VIII). Replaying from empty reproduces the live
// projection (FR-003/SC-005, extended to remote events).
//
// F-1 / SC-006: the projection holds metadata only. An action record carries
// what happened (kind) + a human-readable `detail` derived from METADATA
// (argv, path, reason) — never captured output. There is no field into which
// stdout/stderr/file-bytes/diff could be folded, so the boundary is structural.

import type { RemoteEvent } from "./remote-events";

/** One audited remote action — metadata only (F-1). */
export interface RemoteActionRecord {
  kind: "control" | "command" | "file_read" | "tree" | "refused";
  client_id: string;
  /** Human-readable summary built from metadata (NOT output). */
  detail: string;
  ts: string;
}

/** The remote-session projection the surface renders. */
export interface RemoteSessionProjection {
  armed: boolean;
  operator_id: string | null;
  node_id: string | null;
  attached_clients: string[];
  actions: RemoteActionRecord[];
}

export function initialRemoteSession(): RemoteSessionProjection {
  return { armed: false, operator_id: null, node_id: null, attached_clients: [], actions: [] };
}

/** Fold one remote event into the projection. Pure: returns a new projection. */
export function foldRemoteEvent(
  s: RemoteSessionProjection,
  event: RemoteEvent,
): RemoteSessionProjection {
  switch (event.type) {
    case "remote.armed":
      return { ...s, armed: true, operator_id: event.operator_id, node_id: event.node_id };
    case "remote.disarmed":
      return { ...s, armed: false, operator_id: null, node_id: null, attached_clients: [] };
    case "remote.attached":
      return s.attached_clients.includes(event.client_id)
        ? s
        : { ...s, attached_clients: [...s.attached_clients, event.client_id] };
    case "remote.detached":
      return { ...s, attached_clients: s.attached_clients.filter((c) => c !== event.client_id) };
    case "remote.control":
      return appendAction(s, {
        kind: "control",
        client_id: event.client_id,
        detail: `${event.kind} ${event.ref}`,
        ts: event.ts,
      });
    case "dev_context.command":
      return appendAction(s, {
        kind: "command",
        client_id: event.client_id,
        detail: event.argv.join(" "),
        ts: event.ts,
      });
    case "dev_context.file_read":
      return appendAction(s, {
        kind: "file_read",
        client_id: event.client_id,
        detail: `${event.path} (${event.bytes}b)`,
        ts: event.ts,
      });
    case "dev_context.tree":
      return appendAction(s, {
        kind: "tree",
        client_id: event.client_id,
        detail: event.root,
        ts: event.ts,
      });
    case "dev_context.refused":
      return appendAction(s, {
        kind: "refused",
        client_id: event.client_id,
        detail: `${event.path} — ${event.reason}`,
        ts: event.ts,
      });
    default: {
      const unreachable: never = event;
      throw new Error(`unhandled remote-event: ${JSON.stringify(unreachable)}`);
    }
  }
}

function appendAction(
  s: RemoteSessionProjection,
  action: RemoteActionRecord,
): RemoteSessionProjection {
  return { ...s, actions: [...s.actions, action] };
}

/** Replay a remote log from empty into its session projection. */
export function replayRemote(events: readonly RemoteEvent[]): RemoteSessionProjection {
  let s = initialRemoteSession();
  for (const event of events) s = foldRemoteEvent(s, event);
  return s;
}
