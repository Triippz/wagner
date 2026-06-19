// 011 P7 — resync-on-gap over the typed stream. In-order envelopes fold and
// advance the cursor; a skipped seq is a gap (resync needed); a stale seq is a
// duplicate; streams are independent; resyncTo resets a stream.

import { describe, expect, it } from "vitest";
import type { Envelope } from "../contracts";
import { observe, resyncTo, streamKey, type StreamCursor } from "./resync";

/** Minimal envelope for a given run-stream id + seq (only stream + seq matter). */
function env(stream: string, seq: number): Envelope {
  return {
    schema: "envelope.v1",
    id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    ts: "2026-06-19T00:00:00Z",
    origin: {
      node: "n",
      kind: "system",
      name: "t",
      instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    },
    stream: { type: "run", data: stream },
    seq,
    scope: { user: "u", workspace: "w" },
    payload: { type: "run", data: { type: "finished", data: { run_id: stream, ok: true } } },
  } as Envelope;
}

describe("resync-on-gap", () => {
  it("folds in-order envelopes and advances the cursor", () => {
    const cursor: StreamCursor = {};
    expect(observe(cursor, env("A", 0))).toEqual({ kind: "ok" });
    expect(observe(cursor, env("A", 1))).toEqual({ kind: "ok" });
    expect(observe(cursor, env("A", 2))).toEqual({ kind: "ok" });
    expect(cursor[streamKey({ type: "run", data: "A" })]).toBe(2);
  });

  it("reports a gap when seq skips ahead", () => {
    const cursor: StreamCursor = {};
    expect(observe(cursor, env("A", 0))).toEqual({ kind: "ok" });
    // Missed seq 1 and 2; seq 3 arrives.
    expect(observe(cursor, env("A", 3))).toEqual({ kind: "gap", from: 1, to: 3 });
  });

  it("drops a duplicate (stale) seq", () => {
    const cursor: StreamCursor = {};
    observe(cursor, env("A", 0));
    observe(cursor, env("A", 1));
    expect(observe(cursor, env("A", 1))).toEqual({ kind: "duplicate" });
    expect(observe(cursor, env("A", 0))).toEqual({ kind: "duplicate" });
  });

  it("tracks streams independently", () => {
    const cursor: StreamCursor = {};
    expect(observe(cursor, env("A", 0))).toEqual({ kind: "ok" });
    // Stream B starts fresh at 0 even though A is at 0.
    expect(observe(cursor, env("B", 0))).toEqual({ kind: "ok" });
    expect(observe(cursor, env("B", 1))).toEqual({ kind: "ok" });
    // A gap on B does not touch A.
    expect(observe(cursor, env("B", 5))).toEqual({ kind: "gap", from: 2, to: 5 });
    expect(observe(cursor, env("A", 1))).toEqual({ kind: "ok" });
  });

  it("resyncTo resets a stream cursor so folding resumes in order", () => {
    const cursor: StreamCursor = {};
    observe(cursor, env("A", 0));
    // Rehydrated from a snapshot taken at seq 9.
    resyncTo(cursor, { type: "run", data: "A" }, 9);
    expect(observe(cursor, env("A", 10))).toEqual({ kind: "ok" });
    expect(observe(cursor, env("A", 9))).toEqual({ kind: "duplicate" });
  });
});
