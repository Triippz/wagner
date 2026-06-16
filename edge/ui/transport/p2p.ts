// P2P transport adapter (remote) — T030, FR-002/210.
//
// Folds the attached iroh event stream identically to local: the surface above
// this adapter is unchanged whether events arrive over IPC or P2P. The iroh
// channel is injected as a `P2pChannel` so the adapter is testable headlessly (a
// loopback channel in tests; the real iroh data channel in the browser/mobile
// client). The host frames each event as a channel-tagged envelope, which this
// adapter passes straight through to the surface.

import type { EventStreamTransport, TransportEvent, TransportMessage } from "@wagner/shared/transport";

/** The slice of an iroh data channel the adapter needs — injected for testing. */
export interface P2pChannel {
  /** Subscribe to inbound frames; returns an unsubscribe fn. */
  onFrame(handler: (frame: unknown) => void): () => void;
  /** Send a control frame toward the host. */
  sendFrame(frame: unknown): Promise<void>;
}

export function createP2pTransport(channel: P2pChannel): EventStreamTransport {
  return {
    subscribe(onEvent: (e: TransportEvent) => void) {
      // Inbound frames are arbitrary bytes from the peer. We accept only plain
      // objects here; the surface's parseSurfaceMessage guard does the full
      // discriminant + payload check before any frame reaches the reducer.
      return channel.onFrame((frame) => {
        if (frame === null || typeof frame !== "object") return;
        onEvent(frame as TransportEvent);
      });
    },
    send(message: TransportMessage) {
      return channel.sendFrame(message);
    },
  };
}
