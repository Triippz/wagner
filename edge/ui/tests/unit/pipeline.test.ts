// Phase 2 — linear pipeline model tests (TDD red phase first).
// These tests cover:
//   - compileToWorkflow: a linear list compiles to a valid Workflow
//   - auto-appends a done stage if author didn't add one
//   - gate with pass/fail branch compiles to on_pass/on_fail edges
//   - workflowToPipeline: inverse for loading templates (best-effort)
//   - round-trip: compileToWorkflow(workflowToPipeline(wf)) is still valid
//   - addStage / removeStage / moveStage purity

import { describe, it, expect } from "vitest";
import {
  compileToWorkflow,
  workflowToPipeline,
  addStage,
  removeStage,
  moveStage,
  type PipelineStage,
} from "../../store/pipeline";
import { validateWorkflow, WORKFLOW_SCHEMA, type Workflow } from "../../store/workflow";

// ---- fixtures ---------------------------------------------------------------

const GOAL = "build the thing";

/** 4-stage linear list without an explicit done stage. */
function fourStageList(): PipelineStage[] {
  return [
    { id: "research-1", kind: "research", operativeId: "nova", instruction: "gather context" },
    { id: "plan-1", kind: "plan", operativeId: "cipher" },
    { id: "execute-1", kind: "execute", operativeId: "vex" },
    { id: "review-1", kind: "review", operativeId: "cipher" },
  ];
}

/** 3-stage list with an explicit done. */
function listWithDone(): PipelineStage[] {
  return [
    { id: "plan-1", kind: "plan", operativeId: "cipher" },
    { id: "execute-1", kind: "execute", operativeId: "vex" },
    { id: "done-1", kind: "done", operativeId: null },
  ];
}

/** A gate stage with pass/fail branches. */
function gateList(): PipelineStage[] {
  return [
    { id: "plan-1", kind: "plan", operativeId: "cipher" },
    { id: "gate-1", kind: "gate", operativeId: "cipher",
      branch: { onPass: "done-1", onFail: "fix-1" } },
    { id: "fix-1", kind: "execute", operativeId: "vex" },
    { id: "done-1", kind: "done", operativeId: null },
  ];
}

/**
 * A hand-built minimal valid Workflow for round-trip testing —
 * plan → execute → review(pass→done, fail→execute) shape.
 */
function standardWorkflow(): Workflow {
  return {
    schema: WORKFLOW_SCHEMA,
    root_goal: GOAL,
    nodes: [
      { id: "plan", kind: "plan", operative_id: "cipher" },
      { id: "execute", kind: "execute", operative_id: "vex" },
      { id: "review", kind: "review", operative_id: "cipher" },
      { id: "done", kind: "done" },
    ],
    edges: [
      { from: "plan", to: "execute", when: "always" },
      { from: "execute", to: "review", when: "always" },
      { from: "review", to: "done", when: "on_pass" },
      { from: "review", to: "execute", when: "on_fail" },
    ],
  };
}

// ---- compileToWorkflow -------------------------------------------------------

describe("compileToWorkflow — linear list", () => {
  it("compiles a 4-stage list to a valid Workflow (zero validation errors)", () => {
    const wf = compileToWorkflow(fourStageList(), GOAL);
    expect(validateWorkflow(wf)).toEqual([]);
  });

  it("auto-appends a done stage when the list has none", () => {
    const wf = compileToWorkflow(fourStageList(), GOAL);
    expect(wf.nodes.some((n) => n.kind === "done")).toBe(true);
  });

  it("does NOT double-append done if the list already has one", () => {
    const wf = compileToWorkflow(listWithDone(), GOAL);
    const doneCount = wf.nodes.filter((n) => n.kind === "done").length;
    expect(doneCount).toBe(1);
    expect(validateWorkflow(wf)).toEqual([]);
  });

  it("chains stages with always edges", () => {
    const wf = compileToWorkflow(fourStageList(), GOAL);
    // first stage has an outgoing always edge
    expect(wf.edges.some((e) => e.from === "research-1" && e.when === "always")).toBe(true);
  });

  it("preserves operative_id, instruction, sub_operatives on nodes", () => {
    const wf = compileToWorkflow(fourStageList(), GOAL);
    const node = wf.nodes.find((n) => n.id === "research-1")!;
    expect(node.operative_id).toBe("nova");
    expect(node.instruction).toBe("gather context");
  });

  it("sets root_goal and schema correctly", () => {
    const wf = compileToWorkflow(fourStageList(), GOAL);
    expect(wf.root_goal).toBe(GOAL);
    expect(wf.schema).toBe(WORKFLOW_SCHEMA);
  });
});

describe("compileToWorkflow — gate with pass/fail branch", () => {
  it("compiles a gate's branch to on_pass/on_fail edges and validates", () => {
    const wf = compileToWorkflow(gateList(), GOAL);
    expect(validateWorkflow(wf)).toEqual([]);
  });

  it("emits an on_pass edge from gate to onPass target", () => {
    const wf = compileToWorkflow(gateList(), GOAL);
    expect(
      wf.edges.some((e) => e.from === "gate-1" && e.to === "done-1" && e.when === "on_pass"),
    ).toBe(true);
  });

  it("emits an on_fail edge from gate to onFail target", () => {
    const wf = compileToWorkflow(gateList(), GOAL);
    expect(
      wf.edges.some((e) => e.from === "gate-1" && e.to === "fix-1" && e.when === "on_fail"),
    ).toBe(true);
  });

  it("does not emit an always edge from the gate when branch is set", () => {
    const wf = compileToWorkflow(gateList(), GOAL);
    expect(wf.edges.some((e) => e.from === "gate-1" && e.when === "always")).toBe(false);
  });
});

// ---- workflowToPipeline + round-trip ----------------------------------------

describe("workflowToPipeline", () => {
  it("converts a standard workflow into a PipelineStage array", () => {
    const stages = workflowToPipeline(standardWorkflow());
    expect(stages.length).toBeGreaterThan(0);
    // first stage must be a plan kind (start node)
    expect(stages[0]!.kind).toBe("plan");
  });

  it("round-trips: compileToWorkflow(workflowToPipeline(wf)) is still valid", () => {
    const wf = standardWorkflow();
    const stages = workflowToPipeline(wf);
    const recompiled = compileToWorkflow(stages, wf.root_goal);
    expect(validateWorkflow(recompiled)).toEqual([]);
  });

  it("represents a review branch as branch.onPass / branch.onFail on the stage", () => {
    const stages = workflowToPipeline(standardWorkflow());
    const review = stages.find((s) => s.kind === "review");
    expect(review?.branch?.onPass).toBeDefined();
    expect(review?.branch?.onFail).toBeDefined();
  });
});

// ---- pure array helpers ------------------------------------------------------

describe("addStage", () => {
  it("appends a stage and returns a new array (pure)", () => {
    const original = fourStageList();
    const newStage: PipelineStage = { id: "test-1", kind: "test", operativeId: "cipher" };
    const result = addStage(original, newStage);
    expect(result.length).toBe(original.length + 1);
    expect(result[result.length - 1]).toBe(newStage);
    // original is untouched
    expect(original.length).toBe(4);
  });

  it("can insert at a given index", () => {
    const original = fourStageList();
    const newStage: PipelineStage = { id: "inserted", kind: "scope", operativeId: null };
    const result = addStage(original, newStage, 1);
    expect(result[1]).toBe(newStage);
    expect(result.length).toBe(original.length + 1);
  });
});

describe("removeStage", () => {
  it("removes by id and returns a new array (pure)", () => {
    const original = fourStageList();
    const result = removeStage(original, "plan-1");
    expect(result.length).toBe(3);
    expect(result.find((s) => s.id === "plan-1")).toBeUndefined();
    // original is untouched
    expect(original.length).toBe(4);
  });

  it("returns the same-length array if id not found", () => {
    const original = fourStageList();
    expect(removeStage(original, "ghost").length).toBe(4);
  });
});

describe("moveStage", () => {
  it("moves a stage from index to another (pure)", () => {
    const original = fourStageList();
    // move index 0 (research-1) to index 2
    const result = moveStage(original, 0, 2);
    expect(result[2]!.id).toBe("research-1");
    // original is untouched
    expect(original[0]!.id).toBe("research-1");
  });

  it("returns the same array content for same from/to", () => {
    const original = fourStageList();
    const result = moveStage(original, 1, 1);
    expect(result.map((s) => s.id)).toEqual(original.map((s) => s.id));
  });

  it("handles boundary move (last → first)", () => {
    const original = fourStageList();
    const result = moveStage(original, 3, 0);
    expect(result[0]!.id).toBe("review-1");
    expect(result.length).toBe(4);
  });
});
