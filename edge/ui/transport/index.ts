// Transport adapter selection (T012, R-4, FR-002).
//
// The surface picks its EventStreamTransport adapter at boot from the runtime
// environment: a Tauri desktop shell → the in-process IPC adapter; a plain
// browser/mobile web client → the P2P (iroh) adapter. The reducer above this
// seam is identical in both, so the projection is transport-blind.

import type { EventStreamTransport } from "@wagner/shared/transport";
import { createIpcTransport, type TauriBridge } from "./ipc";
import { createP2pTransport, type P2pChannel } from "./p2p";

export type TransportKind = "ipc" | "p2p";

export interface RuntimeEnv {
  /** True when a Tauri desktop runtime is detected (window.__TAURI_INTERNALS__). */
  hasTauri: boolean;
}

/** Detect the current runtime (browser global or injected env for tests). */
export function detectEnv(): RuntimeEnv {
  const hasTauri =
    typeof globalThis !== "undefined" &&
    // Tauri v2 injects __TAURI_INTERNALS__ into the webview global.
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ !== undefined;
  return { hasTauri };
}

/** Choose which adapter to use for an environment. Pure — easy to test. */
export function selectTransport(env: RuntimeEnv): TransportKind {
  return env.hasTauri ? "ipc" : "p2p";
}

/** What each transport path needs from the host runtime. */
export interface TransportDeps {
  /** The Tauri bridge (desktop IPC path). */
  bridge?: TauriBridge;
  /** The iroh data channel (remote P2P path). */
  channel?: P2pChannel;
}

/** Build the concrete transport for the current (or injected) environment. */
export function createTransport(
  env: RuntimeEnv = detectEnv(),
  deps: TransportDeps = {},
): EventStreamTransport {
  if (selectTransport(env) === "ipc") {
    if (!deps.bridge) throw new Error("IPC transport requires a Tauri bridge");
    return createIpcTransport(deps.bridge);
  }
  if (!deps.channel) throw new Error("P2P transport requires an iroh channel");
  return createP2pTransport(deps.channel);
}

export type { EventStreamTransport } from "@wagner/shared/transport";
