// T006 — Remote event fold + replay (Article VIII, FR-004, SC-006/F-1). RED first.
//
// The remote session is a projection of the append-only log, like everything
// else (Gate VIII). This test asserts:
//   (1) folding the new event kinds yields the expected session projection;
//   (2) replaying the log from empty is byte-identical to the incremental fold
//       (FR-003/SC-005 replay-equals-snapshot, extended to remote events);
//   (3) output payloads are NOT in the projection (F-1, SC-006) — the log
//       records that an action happened + its metadata, never the run-bearing
//       content (stdout/stderr/file bytes/diff).
// T011 implements remote-events.ts + remote-reducer.ts to make this pass.

import { describe, expect, it } from "vitest";
import type { RemoteEvent } from "./remote-events";
import { foldRemoteEvent, initialRemoteSession, replayRemote } from "./remote-reducer";

let seq = 0;
const ev = <T extends RemoteEvent>(e: Omit<T, "event_id" | "run_id" | "ts"> & Partial<RemoteEvent>): T =>
  ({
    event_id: `01J000000000000000000000${(seq++).toString(36).toUpperCase().padStart(2, "0")}`,
    run_id: "01J0000000000000000000RUN",
    ts: "2026-06-16T00:00:00Z",
    ...e,
  }) as T;

const armed = () => ev({ type: "remote.armed", operator_id: "op-1", node_id: "n0de", ticket_id: "tkt-1" });

describe("remote-event fold", () => {
  it("arming sets armed + operator/node", () => {
    const s = foldRemoteEvent(initialRemoteSession(), armed());
    expect(s.armed).toBe(true);
    expect(s.operator_id).toBe("op-1");
    expect(s.node_id).toBe("n0de");
  });

  it("attach adds a client; detach removes it", () => {
    let s = foldRemoteEvent(initialRemoteSession(), armed());
    s = foldRemoteEvent(s, ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-1" }));
    s = foldRemoteEvent(s, ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-2" }));
    expect(s.attached_clients).toEqual(["cli-1", "cli-2"]);
    s = foldRemoteEvent(s, ev({ type: "remote.detached", client_id: "cli-1", reason: "dropped" }));
    expect(s.attached_clients).toEqual(["cli-2"]);
  });

  it("disarm clears arming + attached clients", () => {
    let s = foldRemoteEvent(initialRemoteSession(), armed());
    s = foldRemoteEvent(s, ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-1" }));
    s = foldRemoteEvent(s, ev({ type: "remote.disarmed" }));
    expect(s.armed).toBe(false);
    expect(s.operator_id).toBeNull();
    expect(s.attached_clients).toEqual([]);
  });

  it("control + dev-context actions append metadata-only audit records", () => {
    let s = foldRemoteEvent(initialRemoteSession(), armed());
    s = foldRemoteEvent(s, ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-1" }));
    s = foldRemoteEvent(s, ev({ type: "remote.control", client_id: "cli-1", kind: "answer_permission", ref: "tx-1" }));
    s = foldRemoteEvent(s, ev({ type: "dev_context.command", client_id: "cli-1", argv: ["git", "diff"], cwd: "." }));
    s = foldRemoteEvent(s, ev({ type: "dev_context.file_read", client_id: "cli-1", path: "src/main.rs", bytes: 1234 }));
    s = foldRemoteEvent(s, ev({ type: "dev_context.refused", client_id: "cli-1", path: "/etc/passwd", reason: "out_of_scope" }));
    expect(s.actions).toHaveLength(4);
    expect(s.actions.map((a) => a.kind)).toEqual(["control", "command", "file_read", "refused"]);
    // metadata is present...
    expect(s.actions[1]!.detail).toContain("git diff");
    expect(s.actions[3]!.detail).toContain("out_of_scope");
  });
});

describe("replay equals incremental fold (Article VIII)", () => {
  const log: RemoteEvent[] = [
    armed(),
    ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-1" }),
    ev({ type: "remote.control", client_id: "cli-1", kind: "steer", ref: "go" }),
    ev({ type: "dev_context.command", client_id: "cli-1", argv: ["npm", "test"], cwd: "." }),
    ev({ type: "remote.detached", client_id: "cli-1", reason: "closed" }),
    ev({ type: "remote.disarmed" }),
  ];

  it("replay from empty is byte-identical to step-by-step fold", () => {
    const replayed = replayRemote(log);
    let stepwise = initialRemoteSession();
    for (const e of log) stepwise = foldRemoteEvent(stepwise, e);
    expect(JSON.stringify(replayed)).toBe(JSON.stringify(stepwise));
  });

  it("replay is deterministic — same log, byte-identical projection", () => {
    expect(JSON.stringify(replayRemote(log))).toBe(JSON.stringify(replayRemote(log)));
  });
});

describe("F-1 / SC-006 — no run-bearing content in the projection", () => {
  it("the serialized projection contains no output/content fields", () => {
    let s = foldRemoteEvent(initialRemoteSession(), armed());
    s = foldRemoteEvent(s, ev({ type: "remote.attached", operator_id: "op-1", client_id: "cli-1" }));
    s = foldRemoteEvent(s, ev({ type: "dev_context.command", client_id: "cli-1", argv: ["cat", "secret.txt"], cwd: "." }));
    s = foldRemoteEvent(s, ev({ type: "dev_context.file_read", client_id: "cli-1", path: "secret.txt", bytes: 42 }));
    const json = JSON.stringify(s);
    // The projection may legitimately record argv/path (metadata), but NEVER
    // a field carrying captured output / file contents / diff / transcript.
    for (const banned of ["stdout", "stderr", "output", "contents", "file_contents", "diff", "transcript", "payload"]) {
      expect(json, `projection must not carry a '${banned}' field`).not.toContain(`"${banned}"`);
    }
  });
});
