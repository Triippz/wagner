import { describe, it, expect } from "vitest";
import {
  validateWorkflow,
  isGateOrCheck,
  nextEdgeWhen,
  WORKFLOW_SCHEMA,
  type Workflow,
} from "../../store/workflow";

/** A minimal valid graph: plan → execute → review → done, with a fix-loop. */
function standard(): Workflow {
  return {
    schema: WORKFLOW_SCHEMA,
    root_goal: "build the thing",
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

describe("validateWorkflow", () => {
  it("accepts a well-formed standard graph", () => {
    expect(validateWorkflow(standard())).toEqual([]);
  });

  it("rejects an empty graph", () => {
    const wf = { ...standard(), nodes: [], edges: [] };
    expect(validateWorkflow(wf).length).toBeGreaterThan(0);
  });

  it("flags a duplicate stage id", () => {
    const wf = standard();
    wf.nodes.push({ id: "plan", kind: "execute", operative_id: "vex" });
    expect(validateWorkflow(wf).some((e) => /Duplicate/.test(e))).toBe(true);
  });

  it("flags an edge to an unknown stage", () => {
    const wf = standard();
    wf.edges.push({ from: "plan", to: "ghost", when: "always" });
    expect(validateWorkflow(wf).some((e) => /unknown/.test(e))).toBe(true);
  });

  it("rejects an on_fail edge from a non-check stage", () => {
    const wf = standard();
    wf.edges.push({ from: "plan", to: "done", when: "on_fail" });
    expect(validateWorkflow(wf).some((e) => /On-fail/.test(e))).toBe(true);
  });

  it("requires exactly one start stage", () => {
    const wf = standard();
    // orphan with no incoming edge → two starts
    wf.nodes.push({ id: "orphan", kind: "scope", operative_id: "cipher" });
    wf.edges.push({ from: "orphan", to: "done", when: "always" });
    expect(validateWorkflow(wf).some((e) => /exactly one start/.test(e))).toBe(true);
  });

  it("rejects a Done stage with an outgoing edge", () => {
    const wf = standard();
    wf.edges.push({ from: "done", to: "execute", when: "always" });
    expect(validateWorkflow(wf).some((e) => /Done stage done must have no outgoing/.test(e))).toBe(
      true,
    );
  });

  it("flags a non-Done dead end", () => {
    const wf: Workflow = {
      schema: WORKFLOW_SCHEMA,
      root_goal: "g",
      nodes: [
        { id: "a", kind: "plan" },
        { id: "b", kind: "execute" },
        { id: "done", kind: "done" },
      ],
      edges: [
        { from: "a", to: "b", when: "always" },
        { from: "a", to: "done", when: "always" },
      ],
    };
    // b has no outgoing edge → dead end
    expect(validateWorkflow(wf).some((e) => /dead end/.test(e))).toBe(true);
  });
});

describe("helpers", () => {
  it("classifies gate/check stages", () => {
    expect(isGateOrCheck("gate")).toBe(true);
    expect(isGateOrCheck("review")).toBe(true);
    expect(isGateOrCheck("test")).toBe(true);
    expect(isGateOrCheck("plan")).toBe(false);
    expect(isGateOrCheck("execute")).toBe(false);
  });

  it("cycles edge labels always → pass → fail → always", () => {
    expect(nextEdgeWhen("always")).toBe("on_pass");
    expect(nextEdgeWhen("on_pass")).toBe("on_fail");
    expect(nextEdgeWhen("on_fail")).toBe("always");
  });
});
