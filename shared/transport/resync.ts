// Resync-on-gap for the unified typed event stream (spec 011 P7). Pure, no I/O,
// no UI dependency (Article VIII) — the consumer (the React surface, once it
// folds the typed stream) tracks a per-stream cursor and, when an envelope's seq
// skips ahead, knows it lagged and must rehydrate from a snapshot.
//
// The bus stamps a 0-based, monotonic per-stream `seq` (011 P1), so ordering is
// expressible here without timestamps: the first envelope on a stream is seq 0,
// and each subsequent in-order envelope is exactly `cursor + 1`.

import type { Envelope } from "../contracts";

/** The ordering scope of an envelope (derived from the generated contract). */
export type StreamId = Envelope["stream"];

/** Last in-order seq folded, per stream key. */
export type StreamCursor = Record<string, number>;

/** A stable bookkeeping key for a stream. */
export function streamKey(stream: StreamId): string {
  return `${stream.type}:${stream.data}`;
}

/** What the consumer should do with an observed envelope. */
export type GapDecision =
  | { kind: "ok" } // next in order — fold it (cursor advanced)
  | { kind: "duplicate" } // already folded (seq below cursor) — drop
  | { kind: "gap"; from: number; to: number }; // [from, to) were missed — resync

/**
 * Decide what to do with `envelope` given `cursor`, advancing the cursor in place
 * on in-order delivery. A `gap` means the consumer fell behind (broadcast Lagged)
 * and must resync the stream from a snapshot; a `duplicate` is dropped.
 */
export function observe(cursor: StreamCursor, envelope: Envelope): GapDecision {
  const key = streamKey(envelope.stream);
  const seq = envelope.seq;
  const expected = key in cursor ? cursor[key] + 1 : 0;
  if (seq === expected) {
    cursor[key] = seq;
    return { kind: "ok" };
  }
  if (seq < expected) {
    return { kind: "duplicate" };
  }
  return { kind: "gap", from: expected, to: seq };
}

/**
 * After rehydrating a stream from a snapshot taken at `seq`, reset the cursor so
 * subsequent envelopes fold in order from there. Never rewinds: passing a `seq`
 * below the current cursor (e.g. the gap's `from` instead of the snapshot seq) is
 * clamped, so already-folded envelopes are not replayed as new.
 */
export function resyncTo(cursor: StreamCursor, stream: StreamId, seq: number): void {
  const key = streamKey(stream);
  const current = key in cursor ? cursor[key] : -1;
  cursor[key] = Math.max(current, seq);
}
