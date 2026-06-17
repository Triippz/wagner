import { describe, it, expect } from "vitest";
import {
  activeRun,
  sessionRows,
  applyEvent,
  applyRun,
  applyTransmission,
  answerTransmission,
  openTransmission,
  operativesByDistrict,
  selectOperative,
  selectedOperative,
  applyPanel,
  seedRoster,
  initialState,
} from "../../store/reducer";
import type { WagnerEvent, RunSnapshot, Transmission } from "../../store/types";

function ev(over: Partial<WagnerEvent> = {}): WagnerEvent {
  return {
    schema: "wagner-event.v1",
    event_id: "e1",
    run_id: "r1",
    operative_id: "cipher",
    faction: "architects",
    activity: "edit",
    district: "stacks",
    state: "working",
    message: "editing x",
    handoff_target_operative_id: null,
    ts: "2026-06-13T00:00:01Z",
    ...over,
  };
}

describe("applyEvent", () => {
  it("creates an operative in its district with bubble + state", () => {
    const s = applyEvent(initialState, ev());
    expect(s.operatives.cipher!.district).toBe("stacks");
    expect(s.operatives.cipher!.state).toBe("working");
    expect(s.operatives.cipher!.bubble).toBe("editing x");
  });

  it("seeds the hired roster as idle operatives at the gate", () => {
    const s = seedRoster(initialState, [
      { id: "cipher", name: "Cipher", engine: "claude" },
      { id: "vex", name: "Vex", engine: "codex" },
      { id: "echo", name: "Echo", engine: { endpoint: { base_url: "http://x", model: "m" } } },
    ]);
    expect(Object.keys(s.operatives)).toHaveLength(3);
    expect(s.operatives.cipher!).toMatchObject({
      name: "Cipher",
      faction: "architects",
      district: "gate",
      state: "idle",
    });
    // Non-Claude engines are Forgers.
    expect(s.operatives.vex!.faction).toBe("forgers");
    expect(s.operatives.echo!.faction).toBe("forgers");
  });

  it("seeds operatives whose lastTs any real event supersedes", () => {
    let s = seedRoster(initialState, [{ id: "cipher", name: "Cipher", engine: "claude" }]);
    // A real event (any ISO ts) must win over the seed, not be dropped as stale.
    s = applyEvent(s, ev({ activity: "edit", district: "stacks", state: "working" }));
    expect(s.operatives.cipher!.district).toBe("stacks");
    expect(s.operatives.cipher!.state).toBe("working");
  });

  it("does not clobber an operative that already arrived from an event", () => {
    let s = applyEvent(initialState, ev({ district: "forge", state: "working" }));
    s = seedRoster(s, [{ id: "cipher", name: "Cipher", engine: "claude" }]);
    // The live operative is preserved (not reset to idle/gate).
    expect(s.operatives.cipher!.district).toBe("forge");
    expect(s.operatives.cipher!.state).toBe("working");
  });

  it("updates an existing operative when a newer event arrives", () => {
    let s = applyEvent(initialState, ev());
    s = applyEvent(
      s,
      ev({ activity: "test", district: "forge", state: "working", ts: "2026-06-13T00:00:02Z" })
    );
    expect(s.operatives.cipher!.district).toBe("forge");
  });

  it("ignores a stale (older) event", () => {
    let s = applyEvent(initialState, ev({ ts: "2026-06-13T00:00:05Z", district: "forge" }));
    s = applyEvent(s, ev({ ts: "2026-06-13T00:00:01Z", district: "stacks" }));
    expect(s.operatives.cipher!.district).toBe("forge");
  });

  it("retains the prior bubble when an event omits a message", () => {
    let s = applyEvent(initialState, ev({ message: "first" }));
    s = applyEvent(s, ev({ message: undefined, ts: "2026-06-13T00:00:02Z" }));
    expect(s.operatives.cipher!.bubble).toBe("first");
  });

  it("accumulates a per-operative thinking transcript across events", () => {
    let s = applyEvent(initialState, ev({ message: "planning", activity: "plan" }));
    s = applyEvent(
      s,
      ev({ message: "editing utils", activity: "edit", ts: "2026-06-13T00:00:02Z" })
    );
    const t = s.operatives.cipher!.transcript;
    expect(t).toHaveLength(2);
    expect(t.map((x) => x.message)).toEqual(["planning", "editing utils"]);
    expect(t[1]!.activity).toBe("edit");
  });

  it("does not append a transcript entry for a message-less event", () => {
    let s = applyEvent(initialState, ev({ message: "first" }));
    s = applyEvent(s, ev({ message: undefined, ts: "2026-06-13T00:00:02Z" }));
    expect(s.operatives.cipher!.transcript).toHaveLength(1);
  });
});

describe("applyPanel (LLM-authored agent panels)", () => {
  it("stores a validated panel keyed by operative id", () => {
    const s = applyPanel(initialState, "cipher", {
      title: "t",
      blocks: [{ kind: "text", text: "hi" }],
    });
    expect(s.panels.cipher?.blocks[0]!.kind).toBe("text");
  });

  it("ignores an unrenderable panel (no state change)", () => {
    const s = applyPanel(initialState, "cipher", { blocks: [{ kind: "script" }] });
    expect(s.panels.cipher).toBeUndefined();
  });
});

describe("operative selection", () => {
  it("selects an operative by id and resolves it", () => {
    let s = applyEvent(initialState, ev());
    s = selectOperative(s, "cipher");
    expect(s.selectedOperativeId).toBe("cipher");
    expect(selectedOperative(s)?.id).toBe("cipher");
  });

  it("clears the selection with null", () => {
    let s = selectOperative(applyEvent(initialState, ev()), "cipher");
    s = selectOperative(s, null);
    expect(s.selectedOperativeId).toBeNull();
    expect(selectedOperative(s)).toBeNull();
  });

  it("resolves to null when the selected id has no operative", () => {
    const s = selectOperative(initialState, "ghost");
    expect(selectedOperative(s)).toBeNull();
  });
});

describe("operativesByDistrict", () => {
  it("groups operatives by their district", () => {
    let s = applyEvent(initialState, ev({ operative_id: "cipher", district: "stacks" }));
    s = applyEvent(s, ev({ operative_id: "raze", faction: "forgers", district: "forge" }));
    const g = operativesByDistrict(s);
    expect(g.stacks!.map((o) => o.id)).toEqual(["cipher"]);
    expect(g.forge!.map((o) => o.id)).toEqual(["raze"]);
  });
});

describe("run + transmissions", () => {
  const run: RunSnapshot = {
    schema: "wagner-run.v1",
    run_id: "r1",
    goal: "build",
    status: "running",
    iteration: 2,
    guardrails: {
      max_iterations: 50,
      blocked_timeout_secs: 1800,
      cost: { mode: "cli_usage", used: 1.5 },
    },
  };
  const tx = (over: Partial<Transmission> = {}): Transmission => ({
    schema: "transmission.v1",
    id: "t1",
    subtask_id: "s1",
    kind: "permission",
    prompt: "allow write?",
    options: [
      { id: "yes", label: "Allow" },
      { id: "no", label: "Deny" },
    ],
    raised_at: "2026-06-13T00:00:00Z",
    state: "open",
    ...over,
  });

  it("stores the run snapshot for the HUD", () => {
    const s = applyRun(initialState, run);
    expect(activeRun(s)?.iteration).toBe(2);
  });

  it("keeps concurrent sessions independent, keyed by run id", () => {
    // Two sessions with different ids coexist; updating one never clobbers the
    // other; the first to arrive auto-focuses (acceptance U1, U2).
    const a = applyRun(initialState, { ...run, run_id: "rA", iteration: 1 });
    const ab = applyRun(a, { ...run, run_id: "rB", iteration: 9 });
    expect(Object.keys(ab.runs).sort()).toEqual(["rA", "rB"]);
    expect(ab.runs.rA!.iteration).toBe(1);
    expect(ab.runs.rB!.iteration).toBe(9);
    // First session (rA) auto-focused, so activeRun follows it.
    expect(ab.selectedRunId).toBe("rA");
    expect(activeRun(ab)?.run_id).toBe("rA");
  });

  it("merges live sessions with persisted closed summaries for the rail", () => {
    // One live session (rA) + two closed summaries (rB live-shadowed, rC only on
    // disk). Live wins; result is newest-first by updated_at (acceptance U6).
    const live = applyRun(initialState, {
      ...run,
      run_id: "rA",
      name: "alpha",
      updated_at: "2026-06-17T05:00:00Z",
    });
    const rows = sessionRows(live.runs, [
      { run_id: "rA", name: "stale-alpha", goal: "g", project_dir: "/a", status: "paused", updated_at: "2026-06-17T01:00:00Z" },
      { run_id: "rC", name: "gamma", goal: "g", project_dir: "/c", status: "met", updated_at: "2026-06-17T04:00:00Z" },
    ]);
    expect(rows.map((r) => r.run_id)).toEqual(["rA", "rC"]); // newest-first
    const rA = rows.find((r) => r.run_id === "rA")!;
    expect(rA.live).toBe(true); // live snapshot wins over the stale summary
    expect(rA.name).toBe("alpha");
    expect(rA.dot).toBe("running");
    const rC = rows.find((r) => r.run_id === "rC")!;
    expect(rC.live).toBe(false);
    expect(rC.dot).toBe("done"); // met -> done
  });

  it("stores the run phase and subtasks for the mission bar + inspector", () => {
    const s = applyRun(initialState, {
      ...run,
      phase: "dispatching",
      subtasks: [
        {
          id: "r1-0-0",
          agent_id: "cipher",
          prompt: "write the slugify test",
          state: "running",
          assignment_rationale: "test design is the architect's job",
        },
      ],
    });
    expect(activeRun(s)?.phase).toBe("dispatching");
    expect(activeRun(s)?.subtasks?.[0]!.agent_id).toBe("cipher");
  });

  it("opens and dedups transmissions by id", () => {
    let s = applyTransmission(initialState, tx());
    s = applyTransmission(s, tx({ prompt: "updated?" }));
    expect(s.transmissions).toHaveLength(1);
    expect(s.transmissions[0]!.prompt).toBe("updated?");
    expect(openTransmission(s)?.id).toBe("t1");
  });

  it("answering closes the open transmission", () => {
    let s = applyTransmission(initialState, tx());
    s = answerTransmission(s, "t1", "yes");
    expect(openTransmission(s)).toBeNull();
    expect(s.transmissions[0]!.response).toBe("yes");
  });
});
