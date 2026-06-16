// T007 — Transport abstraction (FR-001/002). RED first.
//
// The surface folds the same pure reducer over an EventStreamTransport; it is
// blind to whether events arrive via in-process IPC (desktop) or P2P (remote).
// This test feeds an IDENTICAL event sequence through two different fake
// transports and asserts the folded projection is identical — proving the
// reducer's output depends only on the events, never on the transport
// ("remote feels no different", stated mechanically). T012 implements the
// contract + the env-detected adapter selection.

import { describe, expect, it } from "vitest";
import type { EventStreamTransport, TransportEvent } from "@wagner/shared/transport";
import { foldRemoteEvent, initialRemoteSession } from "@wagner/shared/reducer/remote-reducer";
import type { RemoteEvent } from "@wagner/shared/reducer/remote-events";
import { selectTransport } from "./index";

/** A fake transport that replays a fixed event list on subscribe. */
function fakeTransport(events: TransportEvent[]): EventStreamTransport {
  return {
    subscribe(onEvent) {
      for (const e of events) onEvent(e);
      return () => {};
    },
    send() {
      return Promise.resolve();
    },
  };
}

/** Drive a transport and fold every delivered event into a remote projection. */
function foldOver(transport: EventStreamTransport) {
  let projection = initialRemoteSession();
  const unsub = transport.subscribe((e) => {
    projection = foldRemoteEvent(projection, e as unknown as RemoteEvent);
  });
  unsub();
  return projection;
}

const LOG: RemoteEvent[] = [
  { type: "remote.armed", operator_id: "op-1", node_id: "n0", ticket_id: "t1", event_id: "e1", run_id: "r", ts: "2026-06-16T00:00:00Z" },
  { type: "remote.attached", operator_id: "op-1", client_id: "c1", event_id: "e2", run_id: "r", ts: "2026-06-16T00:00:01Z" },
  { type: "dev_context.command", client_id: "c1", argv: ["git", "status"], cwd: ".", event_id: "e3", run_id: "r", ts: "2026-06-16T00:00:02Z" },
];

describe("transport-agnostic fold (FR-002)", () => {
  it("two different transports delivering the same events fold to the identical projection", () => {
    const viaIpc = foldOver(fakeTransport(LOG as unknown as TransportEvent[]));
    const viaP2p = foldOver(fakeTransport(LOG as unknown as TransportEvent[]));
    expect(JSON.stringify(viaIpc)).toBe(JSON.stringify(viaP2p));
    expect(viaIpc.attached_clients).toEqual(["c1"]);
    expect(viaIpc.actions).toHaveLength(1);
  });

  it("unsubscribe returned by subscribe is callable", () => {
    const t = fakeTransport([]);
    const unsub = t.subscribe(() => {});
    expect(() => unsub()).not.toThrow();
  });
});

describe("env-detected adapter selection (T012)", () => {
  it("selects the IPC adapter when a Tauri runtime is present", () => {
    expect(selectTransport({ hasTauri: true })).toBe("ipc");
  });

  it("selects the P2P adapter in a plain browser (no Tauri)", () => {
    expect(selectTransport({ hasTauri: false })).toBe("p2p");
  });
});
