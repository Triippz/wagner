// T018a — Unified-surface render (FR-001, US1-AS-6, R1). RED first.
//
// The SAME surface build folds the run view from the carried UI reducer under
// BOTH the IPC adapter (desktop) and a fake P2P adapter (remote) — one codebase,
// env-adaptive. Feeding an identical message sequence through two different
// transports must yield an identical projection (the host-reachability-agnostic
// "remote feels no different", stated mechanically).

import { describe, expect, it } from "vitest";
import type { EventStreamTransport, TransportEvent } from "@wagner/shared/transport";
import { createSurface, type SurfaceMessage } from "./surface";
import { activeRun } from "../store/reducer";

/** A fake transport that replays a fixed message list on subscribe. */
function fakeTransport(messages: SurfaceMessage[]): EventStreamTransport {
  return {
    subscribe(onEvent) {
      for (const m of messages) onEvent(m as unknown as TransportEvent);
      return () => {};
    },
    send() {
      return Promise.resolve();
    },
  };
}

const MESSAGES: SurfaceMessage[] = [
  {
    channel: "run",
    payload: {
      run_id: "r1",
      goal: "build the thing",
      status: "running",
      phase: "dispatching",
      iteration: 1,
      cost: 0.5,
      subtasks: [],
    },
  } as unknown as SurfaceMessage,
  {
    channel: "event",
    payload: {
      schema: "wagner-event.v1",
      event_id: "e1",
      run_id: "r1",
      operative_id: "cipher",
      operative_name: "Cipher",
      faction: "architects",
      activity: "edit",
      district: "stacks",
      state: "working",
      message: "editing main.rs",
      ts: "2026-06-16T00:00:01Z",
    },
  } as unknown as SurfaceMessage,
];

describe("unified surface folds identically across transports (FR-001/002)", () => {
  it("IPC-fake and P2P-fake transports produce the identical projection", () => {
    const viaIpc = createSurface(fakeTransport(MESSAGES));
    const viaP2p = createSurface(fakeTransport(MESSAGES));
    expect(JSON.stringify(viaIpc.getState())).toBe(JSON.stringify(viaP2p.getState()));
  });

  it("renders the run + the operative from the folded stream", () => {
    const surface = createSurface(fakeTransport(MESSAGES));
    const state = surface.getState();
    expect(activeRun(state)?.goal).toBe("build the thing");
    expect(state.operatives.cipher?.name).toBe("Cipher");
    expect(state.operatives.cipher?.bubble).toBe("editing main.rs");
  });

  it("notifies onChange listeners as messages fold in", () => {
    let calls = 0;
    // Build a transport that lets us drive events after subscribe.
    let emit: ((e: TransportEvent) => void) | null = null;
    const transport: EventStreamTransport = {
      subscribe(onEvent) {
        emit = onEvent;
        return () => {};
      },
      send: () => Promise.resolve(),
    };
    const surface = createSurface(transport);
    const unsub = surface.onChange(() => calls++);
    emit!(MESSAGES[0] as unknown as TransportEvent);
    emit!(MESSAGES[1] as unknown as TransportEvent);
    unsub();
    emit!(MESSAGES[0] as unknown as TransportEvent);
    expect(calls).toBe(2); // unsubscribed before the third
  });
});
