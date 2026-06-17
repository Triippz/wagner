// T022 — Voice command name + arg shape (RED-first, step 6 + step 7).
//
// Asserts that cmd.voiceStatus(), cmd.voiceSetEnabled(on), cmd.voiceModelsStatus(),
// and cmd.voiceDownloadModels() call the correct Tauri command names with the
// expected argument shapes. The @tauri-apps/api/core module is mocked so no
// native shell is required.

import { describe, expect, it, vi, beforeEach } from "vitest";

// Capture every invoke() call so assertions can inspect command + args.
const invokes: Array<{ command: string; args: unknown }> = [];

// Per-command realistic response shapes (mirrors Rust return types).
const MOCK_RESPONSES: Record<string, unknown> = {
  voice_status: { enabled: false, ready: false },
  voice_set_enabled: { enabled: false, ready: false },
  voice_models_status: { stt: "absent", tts: "absent" },
  voice_download_models: undefined,
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: unknown) => {
    invokes.push({ command, args: args ?? null });
    const shape = Object.prototype.hasOwnProperty.call(MOCK_RESPONSES, command)
      ? MOCK_RESPONSES[command]
      : null;
    return Promise.resolve(shape);
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

  // Step 7: model download manager commands (RED-first).
  it("voiceModelsStatus() invokes 'voice_models_status' with no args", async () => {
    await cmd.voiceModelsStatus().catch(() => {});
    expect(invokes).toContainEqual({ command: "voice_models_status", args: null });
  });

  it("voiceDownloadModels() invokes 'voice_download_models' with no args", async () => {
    await cmd.voiceDownloadModels().catch(() => {});
    expect(invokes).toContainEqual({ command: "voice_download_models", args: null });
  });
});
