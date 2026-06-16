// T030 — P2P adapter folds the attached stream identically to local (FR-002/210).

import { describe, expect, it } from "vitest";
import { createP2pTransport, type P2pChannel } from "./p2p";
import { createSurface } from "../surfaces/surface";

function loopbackChannel() {
  let handler: ((f: unknown) => void) | null = null;
  const sent: unknown[] = [];
  const channel: P2pChannel = {
    onFrame(h) {
      handler = h;
      return () => {
        handler = null;
      };
    },
    sendFrame(f) {
      sent.push(f);
      return Promise.resolve();
    },
  };
  return { channel, deliver: (f: unknown) => handler?.(f), sent };
}

describe("P2P adapter", () => {
  it("delivers channel frames to the surface and folds them", () => {
    const lb = loopbackChannel();
    const surface = createSurface(createP2pTransport(lb.channel));
    lb.deliver({ channel: "run", payload: { goal: "remote run" } });
    expect(surface.getState().run?.goal).toBe("remote run");
  });

  it("a remote attach folds the SAME projection a local one would", () => {
    const frames = [
      { channel: "run", payload: { goal: "g" } },
      { channel: "event", payload: { schema: "wagner-event.v1", event_id: "e", run_id: "r", operative_id: "cipher", operative_name: "Cipher", faction: "architects", activity: "edit", district: "stacks", state: "working", ts: "2026-06-16T00:00:00Z" } },
    ];
    // Drive the same frames over two independent channels → identical state.
    const a = loopbackChannel();
    const sa = createSurface(createP2pTransport(a.channel));
    frames.forEach(a.deliver);

    const b = loopbackChannel();
    const sb = createSurface(createP2pTransport(b.channel));
    frames.forEach(b.deliver);

    expect(JSON.stringify(sa.getState())).toBe(JSON.stringify(sb.getState()));
  });

  it("routes a control send over the channel", async () => {
    const lb = loopbackChannel();
    const transport = createP2pTransport(lb.channel);
    await transport.send({ kind: "answer_permission", ref: "tx-1", answer: "allow" });
    expect(lb.sent).toContainEqual({ kind: "answer_permission", ref: "tx-1", answer: "allow" });
  });

  it("drops non-object frames — primitive injections never reach the reducer", () => {
    const lb = loopbackChannel();
    const surface = createSurface(createP2pTransport(lb.channel));
    const before = surface.getState();
    // A rogue peer sends a primitive — not an object at all.
    lb.deliver("INJECT");
    lb.deliver(42);
    lb.deliver(null);
    lb.deliver(undefined);
    expect(surface.getState()).toBe(before); // state reference unchanged
  });

  it("drops frames with unknown or missing channel discriminant", () => {
    const lb = loopbackChannel();
    const surface = createSurface(createP2pTransport(lb.channel));
    const before = surface.getState();
    // Valid object shape but unknown channel — should not reach the reducer.
    lb.deliver({ channel: "__proto__", payload: { goal: "poison" } });
    lb.deliver({ channel: "constructor", payload: {} });
    lb.deliver({ payload: { goal: "no-channel" } }); // missing channel field
    lb.deliver({ channel: "event" }); // missing payload
    lb.deliver({ channel: "run", payload: null }); // null payload
    expect(surface.getState()).toBe(before);
  });
});
