import { describe, it, expect } from "vitest";
import { replayRun, foldRunEvent } from "./run-reducer";
import type { RunEvent } from "./run-events";

// T006 (audit F1): the wedge-built event-sourced run spine. The carried edge
// snapshot-persists run state and treats WagnerEvents as a UI projection, so
// Article VIII / FR-003 / SC-005 ("replay log == metadata snapshot") is proven
// here against a NEW run-event log + pure run reducer — not the carried reducer.

function base(event_id: string, ts: string) {
  return { schema: "wagner-run-event.v1" as const, event_id, run_id: "r1", ts };
}

// A representative completed run: created → 2 iterations, 2 cost folds → met.
const completedLog: RunEvent[] = [
  { ...base("e1", "2026-06-15T00:00:00Z"), type: "created", goal: "build slugify", started_at: "2026-06-15T00:00:00Z" },
  { ...base("e2", "2026-06-15T00:00:01Z"), type: "iteration_advanced" },
  { ...base("e3", "2026-06-15T00:00:02Z"), type: "cost_folded", delta: 0.5 },
  { ...base("e4", "2026-06-15T00:00:03Z"), type: "iteration_advanced" },
  { ...base("e5", "2026-06-15T00:00:04Z"), type: "cost_folded", delta: 0.25 },
  { ...base("e6", "2026-06-15T00:05:00Z"), type: "finished", status: "met", halt_reason: null, ended_at: "2026-06-15T00:05:00Z" },
];

describe("run-reducer — event-sourced run metadata (Article VIII / FR-003 / SC-005)", () => {
  it("replays the run-event log from empty into the synced metadata snapshot (FR-006 fields)", () => {
    expect(replayRun(completedLog)).toEqual({
      run_id: "r1",
      goal: "build slugify",
      status: "met",
      halt_reason: null,
      iterations_used: 2,
      cost_used: 0.75,
      started_at: "2026-06-15T00:00:00Z",
      ended_at: "2026-06-15T00:05:00Z",
    });
  });

  it("SC-005: replay-from-empty is byte-identical to the incrementally-folded live snapshot", () => {
    // 'live' = the host's in-memory snapshot, folded one event at a time as the run executes.
    let live = foldRunEvent(null, completedLog[0]!);
    for (const e of completedLog.slice(1)) live = foldRunEvent(live, e);
    expect(JSON.stringify(replayRun(completedLog))).toBe(JSON.stringify(live));
  });

  it("records a guardrail halt with its reason (FR-006 outcome/halt reason)", () => {
    const halted = replayRun([
      completedLog[0]!,
      { ...base("h2", "2026-06-15T00:00:01Z"), type: "iteration_advanced" },
      { ...base("h3", "2026-06-15T00:30:00Z"), type: "finished", status: "halted_guardrail", halt_reason: "cost", ended_at: "2026-06-15T00:30:00Z" },
    ]);
    expect(halted.status).toBe("halted_guardrail");
    expect(halted.halt_reason).toBe("cost");
    expect(halted.ended_at).toBe("2026-06-15T00:30:00Z");
  });

  it("an un-finished (running) run has status 'running' and ended_at null", () => {
    const running = replayRun([completedLog[0]!, completedLog[1]!]);
    expect(running.status).toBe("running");
    expect(running.iterations_used).toBe(1);
    expect(running.ended_at).toBeNull();
  });

  it("throws on a malformed log that does not begin with run.created", () => {
    expect(() => replayRun([{ ...base("x1", "2026-06-15T00:00:00Z"), type: "iteration_advanced" }])).toThrow(/run\.created/);
  });

  it("throws on an empty log (no run)", () => {
    expect(() => replayRun([])).toThrow();
  });

  it("the fold is pure — folding does not mutate the prior snapshot (Article VIII)", () => {
    const a = foldRunEvent(null, completedLog[0]!);
    const b = foldRunEvent(a, completedLog[1]!); // iteration_advanced
    expect(a.iterations_used).toBe(0); // 'a' is untouched
    expect(b.iterations_used).toBe(1);
    expect(a).not.toBe(b);
  });
});
