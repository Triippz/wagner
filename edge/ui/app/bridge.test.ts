// T022 — Voice command name + arg shape (RED-first, step 6).
//
// Asserts that cmd.voiceStatus() and cmd.voiceSetEnabled(on) call the correct
// Tauri command names with the expected argument shapes. The @tauri-apps/api/core
// module is mocked so no native shell is required.

import { describe, expect, it, vi, beforeEach } from "vitest";

// Capture every invoke() call so assertions can inspect command + args.
const invokes: Array<{ command: string; args: unknown }> = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: unknown) => {
    invokes.push({ command, args: args ?? null });
    return Promise.resolve(null);
  },
  listen: () => Promise.resolve(() => {}),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => {}),
}));

// Import after the mock is registered so the module-level invoke binding picks it up.
const { cmd } = await import("./bridge");

beforeEach(() => {
  invokes.length = 0;
});

describe("cmd voice commands", () => {
  it("voiceStatus() invokes 'voice_status' with no args", async () => {
    await cmd.voiceStatus().catch(() => {});
    expect(invokes).toContainEqual({ command: "voice_status", args: null });
  });

  it("voiceSetEnabled(true) invokes 'voice_set_enabled' with { on: true }", async () => {
    await cmd.voiceSetEnabled(true).catch(() => {});
    expect(invokes).toContainEqual({ command: "voice_set_enabled", args: { on: true } });
  });

  it("voiceSetEnabled(false) invokes 'voice_set_enabled' with { on: false }", async () => {
    await cmd.voiceSetEnabled(false).catch(() => {});
    expect(invokes).toContainEqual({ command: "voice_set_enabled", args: { on: false } });
  });
});
