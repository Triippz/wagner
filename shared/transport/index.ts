// EventStreamTransport — the contract the surface folds over (R-4, FR-002).
//
// The React surface depends ONLY on this interface + the shared reducer; it is
// blind to whether events arrive via in-process IPC (desktop) or P2P (remote).
// Two adapters implement it: edge/ui/transport/ipc.ts and .../p2p.ts. The
// concrete shape (event payloads, send messages) is defined test-first in T012.

/** A delivered run event — the reducer's input, transport-agnostic. */
export type TransportEvent = Record<string, unknown>;

/** A control/command message sent back toward the host. */
export type TransportMessage = Record<string, unknown>;

export interface EventStreamTransport {
  /** Subscribe to the host event stream; returns an unsubscribe fn. */
  subscribe(onEvent: (event: TransportEvent) => void): () => void;
  /** Send a control message toward the host (answer-permission, steer, …). */
  send(message: TransportMessage): Promise<void>;
}
