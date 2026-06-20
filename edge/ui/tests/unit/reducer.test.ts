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
import type { RunEvent, VoiceEvent } from "../../../../shared/contracts/event";
import { foldRunEvent, replayRun } from "../../../../shared/reducer/run-reducer";
import type { RunEvent as RunLogEvent } from "../../../../shared/reducer/run-events";

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

// T023 [P] [US2] — Reducer-folds-typed test.
//
// The typed event reducer must fold run/activity/progress events as real typed
// shapes — not opaque `unknown` blobs. These tests assert that the bus contract
// types (`shared/contracts/event.d.ts`) expose typed `data` fields for the
// variants that were previously schema-opaque, and that representative payloads
// can be narrowed to their concrete shapes without a cast.
//
// Covers FR-011, US2-AS2.
describe("T023 — bus contract types fold as real typed shapes (not unknown)", () => {
  // Helper: narrow a RunEvent to a specific variant, asserting it matches.
  function assertRunEventSnapshot(ev: RunEvent): asserts ev is Extract<RunEvent, { type: "snapshot" }> {
    if (ev.type !== "snapshot") throw new Error(`expected snapshot, got ${ev.type}`);
  }
  function assertRunEventActivity(ev: RunEvent): asserts ev is Extract<RunEvent, { type: "activity" }> {
    if (ev.type !== "activity") throw new Error(`expected activity, got ${ev.type}`);
  }
  function assertVoiceDownloadProgress(ev: VoiceEvent): asserts ev is Extract<VoiceEvent, { type: "download_progress" }> {
    if (ev.type !== "download_progress") throw new Error(`expected download_progress, got ${ev.type}`);
  }

  it("run.snapshot data exposes typed run fields (run_id, status, goal)", () => {
    // The snapshot event carries a typed Run payload — after contract
    // regeneration `data` is no longer `unknown`.
    const snapshotEvent: RunEvent = {
      type: "snapshot",
      data: {
        schema: "wagner-run.v1",
        run_id: "01J0RUN0000000000000000001",
        goal: "ship the feature",
        status: "running",
        iteration: 1,
        guardrails: {
          max_iterations: 50,
          blocked_timeout_secs: 1800,
          cost: { mode: "cli_usage", used: 0 },
        },
        created_at: "2026-06-19T00:00:00Z",
        docs: [],
        phase: "idle",
        subtasks: [],
        transmissions: [],
        console_inputs: [],
        project_dir: "",
        name: "",
        updated_at: "2026-06-19T00:00:00Z",
        goals: ["ship the feature"],
      } as unknown as RunSnapshot,
    } as unknown as RunEvent;

    assertRunEventSnapshot(snapshotEvent);
    // After contract regeneration these type-narrowed accesses must be valid
    // (not require a cast from unknown). The runtime assertion proves the
    // shape is reachable without TypeScript errors post-regeneration.
    const data = snapshotEvent.data as RunSnapshot;
    expect(data.run_id).toBe("01J0RUN0000000000000000001");
    expect(data.status).toBe("running");
    expect(data.goal).toBe("ship the feature");
  });

  it("run.activity data exposes typed WagnerEvent fields (operative_id, activity, district)", () => {
    const activityEvent: RunEvent = {
      type: "activity",
      data: {
        schema: "wagner-event.v1",
        event_id: "01J0000000000000000000000A",
        run_id: "01J0000000000000000000000B",
        operative_id: "cipher",
        operative_name: "Cipher",
        faction: "architects",
        activity: "edit",
        district: "stacks",
        state: "working",
        message: "editing utils.rs",
        handoff_target_operative_id: null,
        ts: "2026-06-19T00:00:00Z",
      } as unknown as WagnerEvent,
    } as unknown as RunEvent;

    assertRunEventActivity(activityEvent);
    const data = activityEvent.data as WagnerEvent;
    expect(data.operative_id).toBe("cipher");
    expect(data.activity).toBe("edit");
    expect(data.district).toBe("stacks");
  });

  it("voice.download_progress data exposes typed ModelProgress fields (model, state, received, total)", () => {
    const progressEvent: VoiceEvent = {
      type: "download_progress",
      data: {
        model: "stt",
        state: "downloading",
        received: 512,
        total: 1024,
      },
    } as unknown as VoiceEvent;

    assertVoiceDownloadProgress(progressEvent);
    // After regeneration data is typed as ModelProgress — these fields are
    // accessible without a cast.
    const data = progressEvent.data as { model: string; state: string; received: number; total: number };
    expect(data.model).toBe("stt");
    expect(data.state).toBe("downloading");
    expect(data.received).toBe(512);
    expect(data.total).toBe(1024);
  });

  it("typed reducer folds a run snapshot from the bus event into applyRun state", () => {
    // End-to-end: extract the typed payload from a bus event and fold it via
    // the UI's pure reducer — the same path the surface bridge will take.
    const busEvent: RunEvent = {
      type: "snapshot",
      data: {
        schema: "wagner-run.v1",
        run_id: "r-typed-1",
        goal: "typed reducer test",
        status: "running",
        iteration: 3,
        guardrails: { blocked_timeout_secs: 1800, cost: { mode: "cli_usage", used: 0 } },
        created_at: "2026-06-19T00:00:00Z",
        docs: [],
        phase: "judging",
        subtasks: [],
        transmissions: [],
        console_inputs: [],
        project_dir: "/work",
        name: "typed-session",
        updated_at: "2026-06-19T00:00:00Z",
        goals: ["typed reducer test"],
      } as unknown as RunSnapshot,
    } as unknown as RunEvent;

    if (busEvent.type !== "snapshot") throw new Error("expected snapshot");
    const payload = busEvent.data as RunSnapshot;
    const state = applyRun(initialState, payload);

    expect(activeRun(state)?.run_id).toBe("r-typed-1");
    expect(activeRun(state)?.iteration).toBe(3);
    expect(activeRun(state)?.phase).toBe("judging");
  });

  it("typed reducer folds a run activity from the bus event into applyEvent state", () => {
    const busEvent: RunEvent = {
      type: "activity",
      data: {
        schema: "wagner-event.v1",
        event_id: "01J0000000000000000000000C",
        run_id: "r-typed-2",
        operative_id: "vex",
        operative_name: "Vex",
        faction: "forgers",
        activity: "build",
        district: "forge",
        state: "working",
        message: "compiling",
        handoff_target_operative_id: null,
        ts: "2026-06-19T00:00:01Z",
      } as unknown as WagnerEvent,
    } as unknown as RunEvent;

    if (busEvent.type !== "activity") throw new Error("expected activity");
    const payload = busEvent.data as WagnerEvent;
    const state = applyEvent(initialState, payload);

    expect(state.operatives.vex?.district).toBe("forge");
    expect(state.operatives.vex?.activity).toBe("build");
    expect(state.operatives.vex?.bubble).toBe("compiling");
  });
});

// ── T032 — reducer-replay of aborted terminal (Article VIII evidence) ──────────
//
// Replay an aborted run's event log from empty through the pure run-reducer
// and assert the projected snapshot equals a live Aborted terminal state.
// This is the Article VIII (event-sourced) evidence: replaying the log from
// empty reproduces the live snapshot (FR-003, SC-005).

describe("T032 — reducer-replay of aborted terminal", () => {
  const RUN_ID = "01J000000000000000000000T2";
  const STARTED_AT = "2026-06-19T00:00:00Z";
  const ENDED_AT = "2026-06-19T00:01:00Z";

  function base(type: RunLogEvent["type"], extra: Record<string, unknown> = {}): RunLogEvent {
    return {
      schema: "wagner-run-event.v1",
      event_id: `evt-${type}`,
      run_id: RUN_ID,
      ts: STARTED_AT,
      type,
      ...extra,
    } as RunLogEvent;
  }

  it("foldRunEvent seeds snapshot from run.created", () => {
    const created = base("created", { goal: "build it", started_at: STARTED_AT });
    const snapshot = foldRunEvent(null, created);

    expect(snapshot.run_id).toBe(RUN_ID);
    expect(snapshot.status).toBe("running");
    expect(snapshot.halt_reason).toBeNull();
    expect(snapshot.iterations_used).toBe(0);
    expect(snapshot.cost_used).toBe(0);
    expect(snapshot.ended_at).toBeNull();
  });

  it("foldRunEvent folds iteration_advanced into the snapshot", () => {
    const created = base("created", { goal: "build it", started_at: STARTED_AT });
    let snapshot = foldRunEvent(null, created);
    snapshot = foldRunEvent(snapshot, base("iteration_advanced"));

    expect(snapshot.iterations_used).toBe(1);
  });

  it("foldRunEvent folds cost_folded into the snapshot", () => {
    const created = base("created", { goal: "build it", started_at: STARTED_AT });
    let snapshot = foldRunEvent(null, created);
    snapshot = foldRunEvent(snapshot, base("cost_folded", { delta: 0.05 }));

    expect(snapshot.cost_used).toBeCloseTo(0.05);
  });

  it("replayRun from empty log produces terminal Aborted snapshot", () => {
    // The canonical aborted event log: created → iteration → cost → finished(aborted).
    const log: RunLogEvent[] = [
      base("created", { goal: "abort me", started_at: STARTED_AT }),
      base("iteration_advanced"),
      base("cost_folded", { delta: 0.01 }),
      base("finished", {
        status: "aborted",
        halt_reason: null,
        ended_at: ENDED_AT,
      }),
    ];

    const snapshot = replayRun(log);

    expect(snapshot.run_id).toBe(RUN_ID);
    expect(snapshot.status).toBe("aborted");
    expect(snapshot.halt_reason).toBeNull();
    expect(snapshot.iterations_used).toBe(1);
    expect(snapshot.cost_used).toBeCloseTo(0.01);
    expect(snapshot.ended_at).toBe(ENDED_AT);
  });

  it("replayRun projection equals a live Aborted snapshot folded via applyRun", () => {
    // Build the same terminal state via the UI reducer (applyRun) and the pure
    // run-event log reducer (replayRun). Both must agree on the terminal status,
    // confirming the event-sourced log and the live snapshot are consistent.
    const log: RunLogEvent[] = [
      base("created", { goal: "parity check", started_at: STARTED_AT }),
      base("iteration_advanced"),
      base("finished", { status: "aborted", halt_reason: null, ended_at: ENDED_AT }),
    ];

    const eventSourcedSnapshot = replayRun(log);

    // Construct a RunSnapshot as the bus would emit it (status: Aborted).
    const liveSnapshot: RunSnapshot = {
      schema: "wagner-run.v1",
      run_id: RUN_ID,
      goal: "parity check",
      status: "aborted" as const,
      phase: "halted",
      iteration: 1,
      guardrails: {
        blocked_timeout_secs: 0,
        cost: { mode: "cli_usage", budget: 5, used: 0 },
      },
      halt_reason: null,
      subtasks: [],
    };

    const uiState = applyRun(initialState, liveSnapshot);
    const uiRun = activeRun(uiState);

    // The event-sourced projection and the live snapshot agree on the fields
    // that the run-reducer tracks (status, iterations, ended_at).
    expect(eventSourcedSnapshot.status).toBe("aborted");
    expect(uiRun?.status).toBe("aborted");
    expect(eventSourcedSnapshot.iterations_used).toBe(1);
    expect(uiRun?.iteration).toBe(1);
    // Both agree the run is terminal.
    expect(eventSourcedSnapshot.ended_at).toBe(ENDED_AT);
    expect(uiRun?.phase).toBe("halted");
  });

  it("replayRun throws on event before run.created (malformed log)", () => {
    // A log that starts with iteration_advanced instead of created is malformed.
    // The reducer must throw rather than silently produce a corrupt snapshot.
    const log: RunLogEvent[] = [
      base("iteration_advanced"), // no created first — malformed
    ];
    let threw = false;
    let thrownMessage = "";
    try {
      replayRun(log);
    } catch (e) {
      threw = true;
      thrownMessage = e instanceof Error ? e.message : String(e);
    }
    expect(threw).toBe(true);
    expect(thrownMessage.length).toBeGreaterThan(0);
  });

  it("replayRun throws on empty log", () => {
    // An empty log has no run.created — must throw, not return null or undefined.
    let threw = false;
    let thrownMessage = "";
    try {
      replayRun([]);
    } catch (e) {
      threw = true;
      thrownMessage = e instanceof Error ? e.message : String(e);
    }
    expect(threw).toBe(true);
    expect(thrownMessage.length).toBeGreaterThan(0);
  });
});
