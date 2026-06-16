// T021 — IPC adapter folds the carried Tauri channels (FR-002).
//
// Drives the adapter with a FAKE Tauri bridge (no live Tauri): asserts it
// subscribes to the three host channels, normalises each into a channel-tagged
// envelope, and routes control sends to the carried commands.

import { describe, expect, it } from "vitest";
import { createIpcTransport, type TauriBridge } from "./ipc";

function fakeBridge() {
  const handlers: Record<string, (p: unknown) => void> = {};
  const invokes: Array<{ command: string; args: unknown }> = [];
  let unlistened = 0;
  const bridge: TauriBridge = {
    listen(channel, handler) {
      handlers[channel] = handler;
      return Promise.resolve(() => {
        unlistened++;
      });
    },
    invoke(command, args) {
      invokes.push({ command, args });
      return Promise.resolve(null);
    },
  };
  return { bridge, handlers, invokes, unlistenedCount: () => unlistened };
}

describe("IPC adapter", () => {
  it("subscribes to the three host channels and tags delivered payloads", async () => {
    const f = fakeBridge();
    const transport = createIpcTransport(f.bridge);
    const received: unknown[] = [];
    transport.subscribe((e) => received.push(e));
    // let the listen() promises resolve
    await Promise.resolve();
    await Promise.resolve();

    f.handlers["wagner://run"]?.({ goal: "x" });
    f.handlers["wagner://event"]?.({ operative_id: "cipher" });

    expect(received).toContainEqual({ channel: "run", payload: { goal: "x" } });
    expect(received).toContainEqual({ channel: "event", payload: { operative_id: "cipher" } });
  });

  it("routes a control send to the carried Tauri command", async () => {
    const f = fakeBridge();
    const transport = createIpcTransport(f.bridge);
    await transport.send({ kind: "steer", text: "go" });
    expect(f.invokes).toContainEqual({
      command: "wagner_steer",
      args: { kind: "steer", text: "go" },
    });
  });

  it("unsubscribe unlistens every channel", async () => {
    const f = fakeBridge();
    const transport = createIpcTransport(f.bridge);
    const unsub = transport.subscribe(() => {});
    await Promise.resolve();
    await Promise.resolve();
    unsub();
    expect(f.unlistenedCount()).toBe(3);
  });

  it("maps an unknown kind to wagner_control (allowlist defence)", async () => {
    const f = fakeBridge();
    const transport = createIpcTransport(f.bridge);
    // kind not in the permitted set → must not build an arbitrary command name
    await transport.send({ kind: "DROP_TABLE", text: "pwned" });
    expect(f.invokes[0]?.command).toBe("wagner_control");
  });

  it("maps each permitted kind to its exact Tauri command", async () => {
    const cases: Array<{ kind: string; expected: string }> = [
      { kind: "answer_permission", expected: "wagner_answer_permission" },
      { kind: "steer", expected: "wagner_steer" },
      { kind: "abort", expected: "wagner_abort" },
      { kind: "control", expected: "wagner_control" },
    ];
    for (const { kind, expected } of cases) {
      const f = fakeBridge();
      const transport = createIpcTransport(f.bridge);
      await transport.send({ kind });
      expect(f.invokes[0]?.command).toBe(expected);
    }
  });
});
