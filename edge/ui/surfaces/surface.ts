// Unified surface core (T020, FR-001/002, R1).
//
// ONE codebase folds the host event stream into UI state over the transport
// abstraction — blind to whether the transport is in-process IPC (desktop) or
// P2P (remote). The same `createSurface` runs in the Tauri desktop shell, a
// browser, and mobile web; only the injected transport differs. This is the
// mechanical core behind "remote feels no different": identical events ⇒
// identical projection, regardless of transport (proven at the reducer level in
// T007 and at the surface level in T018a).

import type { EventStreamTransport, TransportMessage } from "@wagner/shared/transport";
import {
  applyEvent,
  applyRun,
  applyTransmission,
  initialState,
  type WagnerState,
} from "../store/reducer";
import type { RunSnapshot, Transmission, WagnerEvent } from "../store/types";

/** A delivered message tagged by channel so the surface routes it to the right
 *  reducer. Both transports wrap host output in this envelope. */
export type SurfaceMessage =
  | { channel: "event"; payload: WagnerEvent }
  | { channel: "run"; payload: RunSnapshot }
  | { channel: "transmission"; payload: Transmission };

const VALID_CHANNELS = new Set(["event", "run", "transmission"]);

/**
 * Runtime guard for inbound frames (P2P or IPC). Any peer can send arbitrary
 * bytes; only frames that carry a known `channel` discriminant and a non-null
 * `payload` are promoted to `SurfaceMessage`. Invalid frames return `null` and
 * are silently dropped before reaching the reducer.
 */
export function parseSurfaceMessage(raw: unknown): SurfaceMessage | null {
  if (raw === null || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  const channel = r["channel"];
  if (typeof channel !== "string" || !VALID_CHANNELS.has(channel)) return null;
  const payload = r["payload"];
  if (payload === null || typeof payload !== "object") return null;
  // Type system: channel membership + payload presence establishes the union
  // variant. The reducer validates domain invariants (schema, required fields)
  // before mutating state, so we don't duplicate that logic here.
  return { channel, payload } as SurfaceMessage;
}

export interface Surface {
  /** Current folded UI state. */
  getState(): WagnerState;
  /** Subscribe to state changes (for the React render layer). Returns unsub. */
  onChange(listener: (state: WagnerState) => void): () => void;
  /** Send a control message toward the host (answer-permission, steer, …). */
  send(message: TransportMessage): Promise<void>;
  /** Tear down the transport subscription. */
  dispose(): void;
}

function routeMessage(state: WagnerState, msg: SurfaceMessage): WagnerState {
  switch (msg.channel) {
    case "event":
      return applyEvent(state, msg.payload);
    case "run":
      return applyRun(state, msg.payload);
    case "transmission":
      return applyTransmission(state, msg.payload);
    default: {
      const unreachable: never = msg;
      throw new Error(`unknown surface channel: ${JSON.stringify(unreachable)}`);
    }
  }
}

/** Build a surface over a transport. The surface is transport-blind: it folds
 *  whatever the transport delivers through the carried UI reducer. */
export function createSurface(transport: EventStreamTransport): Surface {
  let state = initialState;
  const listeners = new Set<(s: WagnerState) => void>();

  const unsubscribe = transport.subscribe((raw) => {
    const msg = parseSurfaceMessage(raw);
    if (!msg) return; // drop invalid / unexpected frames
    state = routeMessage(state, msg);
    for (const l of listeners) l(state);
  });

  return {
    getState: () => state,
    onChange(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    send: (message) => transport.send(message),
    dispose: () => unsubscribe(),
  };
}
