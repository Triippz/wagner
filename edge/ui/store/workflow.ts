// Workflow model + client-side validation (Phase E).
//
// Mirrors the Rust `orchestrator::workflow` model 1:1 so a graph authored in the
// builder serializes straight to the host `Workflow` and passes the same checks.
// `validateWorkflow` reproduces the Rust `Workflow::validate` rules deterministically
// so the builder can block Launch before round-tripping to the host.

export type StageKind =
  | "research"
  | "aggregate"
  | "scope"
  | "gate"
  | "plan"
  | "interrogate"
  | "tdd"
  | "execute"
  | "review"
  | "test"
  | "done";

export type EdgeWhen = "always" | "on_pass" | "on_fail";

export type GateMode = "human" | "auto";

export interface WorkflowNode {
  id: string;
  kind: StageKind;
  operative_id?: string | null;
  instruction?: string | null;
  gate_mode?: GateMode | null;
  sub_operatives?: string[];
}

export interface WorkflowEdge {
  from: string;
  to: string;
  when: EdgeWhen;
  max_traversals?: number | null;
}

export interface Workflow {
  schema: string;
  root_goal: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

export interface NamedTemplate {
  name: string;
  description: string;
  workflow: Workflow;
}

export const WORKFLOW_SCHEMA = "workflow.v1";

/** Stages that decide pass/fail (and so may have `on_fail` out-edges). */
export const isGateOrCheck = (kind: StageKind): boolean =>
  kind === "gate" || kind === "review" || kind === "test";

/** Human-readable stage labels for the palette/canvas. */
export const STAGE_LABELS: Record<StageKind, string> = {
  research: "Research",
  aggregate: "Aggregate",
  scope: "Scope",
  gate: "Gate",
  plan: "Plan",
  interrogate: "Interrogate",
  tdd: "TDD",
  execute: "Execute",
  review: "Review",
  test: "Test",
  done: "Done",
};

/** Every stage kind, in palette order. */
export const ALL_STAGES: StageKind[] = [
  "research",
  "aggregate",
  "scope",
  "gate",
  "plan",
  "interrogate",
  "tdd",
  "execute",
  "review",
  "test",
  "done",
];

/**
 * Validate a workflow graph. Returns a list of human-readable errors; an empty
 * list means the graph is launchable. Mirrors `Workflow::validate` in Rust.
 */
export function validateWorkflow(wf: Workflow): string[] {
  const errors: string[] = [];
  if (wf.nodes.length === 0) {
    return ["Workflow must have at least one stage."];
  }

  // unique ids
  const ids = new Set<string>();
  for (const n of wf.nodes) {
    if (ids.has(n.id)) errors.push(`Duplicate stage id: ${n.id}`);
    ids.add(n.id);
  }

  // edges reference real nodes
  for (const e of wf.edges) {
    if (!ids.has(e.from)) errors.push(`Edge from unknown stage: ${e.from}`);
    if (!ids.has(e.to)) errors.push(`Edge to unknown stage: ${e.to}`);
  }

  const kindOf = new Map(wf.nodes.map((n) => [n.id, n.kind]));

  // on_fail only from a gate/check
  for (const e of wf.edges) {
    if (e.when === "on_fail") {
      const k = kindOf.get(e.from);
      if (k && !isGateOrCheck(k)) {
        errors.push(`On-fail edge must start at a Gate/Review/Test, not ${STAGE_LABELS[k]} (${e.from}).`);
      }
    }
  }

  // exactly one start (no incoming edges)
  const hasIncoming = new Set(wf.edges.map((e) => e.to));
  const starts = wf.nodes.filter((n) => !hasIncoming.has(n.id)).map((n) => n.id);
  if (starts.length !== 1) {
    errors.push(`Workflow needs exactly one start stage (no incoming edge); found ${starts.length}.`);
  }

  // >=1 Done; Done has no out-edge; non-Done has >=1 out-edge
  const hasOutgoing = new Set(wf.edges.map((e) => e.from));
  let anyDone = false;
  for (const n of wf.nodes) {
    if (n.kind === "done") {
      anyDone = true;
      if (hasOutgoing.has(n.id)) errors.push(`Done stage ${n.id} must have no outgoing edge.`);
    } else if (!hasOutgoing.has(n.id)) {
      errors.push(`Stage ${n.id} has no outgoing edge (dead end).`);
    }
  }
  if (!anyDone) errors.push("Workflow needs at least one Done stage.");

  // a Done is reachable from the start
  if (starts.length === 1 && !reachesDone(wf, starts[0]!)) {
    errors.push("No Done stage is reachable from the start.");
  }

  return errors;
}

function reachesDone(wf: Workflow, start: string): boolean {
  const stack = [start];
  const seen = new Set<string>();
  const out = new Map<string, string[]>();
  for (const e of wf.edges) {
    const arr = out.get(e.from) ?? [];
    arr.push(e.to);
    out.set(e.from, arr);
  }
  const kindOf = new Map(wf.nodes.map((n) => [n.id, n.kind]));
  while (stack.length) {
    const id = stack.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    if (kindOf.get(id) === "done") return true;
    for (const to of out.get(id) ?? []) stack.push(to);
  }
  return false;
}

/** Cycle an edge label Always → Pass → Fail → Always (builder affordance). */
export function nextEdgeWhen(when: EdgeWhen): EdgeWhen {
  return when === "always" ? "on_pass" : when === "on_pass" ? "on_fail" : "always";
}

export const EDGE_LABEL: Record<EdgeWhen, string> = {
  always: "→",
  on_pass: "pass",
  on_fail: "fail",
};
