// IPC transport adapter (desktop) — T021, FR-002.
//
// Wraps the carried Tauri command/event surface so the desktop surface folds the
// host log locally. The Tauri `listen`/`invoke` boundary is injected so the
// adapter is testable headlessly (a fake event source in tests; the real
// `@tauri-apps/api` binding in the shell). The host emits on three channels
// (`wagner://event|run|transmission`); the adapter normalises them into the
// channel-tagged SurfaceMessage envelope the surface folds.

import type { EventStreamTransport, TransportEvent, TransportMessage } from "@wagner/shared/transport";

/** The slice of the Tauri API the adapter needs — injected for testability. */
export interface TauriBridge {
  /** Subscribe to a Tauri event channel; resolves to an unlisten fn. */
  listen(channel: string, handler: (payload: unknown) => void): Promise<() => void>;
  /** Invoke a Tauri command (control messages back to the host). */
  invoke(command: string, args: Record<string, unknown>): Promise<unknown>;
}

const CHANNELS: Array<{ tauri: string; channel: "event" | "run" | "transmission" }> = [
  { tauri: "wagner://event", channel: "event" },
  { tauri: "wagner://run", channel: "run" },
  { tauri: "wagner://transmission", channel: "transmission" },
];

export function createIpcTransport(bridge: TauriBridge): EventStreamTransport {
  const unlisteners: Array<() => void> = [];
  return {
    subscribe(onEvent: (e: TransportEvent) => void) {
      for (const { tauri, channel } of CHANNELS) {
        bridge
          .listen(tauri, (payload) => onEvent({ channel, payload } as unknown as TransportEvent))
          .then((un) => unlisteners.push(un))
          .catch((err: unknown) => {
            // Transport dead — surface cannot receive host events on this channel.
            // Log so the error is traceable; callers can detect the dead state via
            // a missing run snapshot or stale operative positions.
            console.error(`[ipc] listen failed on "${tauri}":`, err);
          });
      }
      return () => {
        for (const un of unlisteners.splice(0)) un();
      };
    },
    send(message: TransportMessage) {
      // Control messages map to the REAL Tauri command names (not `wagner_${kind}`
      // — the host registers bare `steer`/`abort`/`answer_transmission`). This is
      // also the allowlist: a kind with no mapping is dropped, so no arbitrary
      // command name can reach `invoke`.
      const COMMAND_BY_KIND: Record<string, string> = {
        steer: "steer",
        abort: "abort",
        answer_permission: "answer_transmission",
      };
      const rawKind = (message as { kind?: string }).kind;
      const command = typeof rawKind === "string" ? COMMAND_BY_KIND[rawKind] : undefined;
      if (!command) {
        console.warn("[ipc] dropping control message with unmapped kind:", rawKind);
        return Promise.resolve();
      }
      return bridge.invoke(command, message as Record<string, unknown>).then(() => {});
    },
  };
}
