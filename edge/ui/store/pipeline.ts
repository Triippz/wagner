// Linear pipeline model — a view over the existing Workflow graph (Phase 2).
//
// A linear list of PipelineStages compiles deterministically to the existing
// `Workflow` type (chain of `always` edges, with `on_pass`/`on_fail` when a
// gate/check stage has a `branch`). The Rust executor and `validateWorkflow`
// are reused verbatim — this is purely a model layer, no UI, no .rs changes.

import {
  WORKFLOW_SCHEMA,
  isGateOrCheck,
  validateWorkflow,
  type StageKind,
  type Workflow,
  type WorkflowEdge,
  type WorkflowNode,
} from "./workflow";

// ---- types ------------------------------------------------------------------

/** A stage in the linear pipeline editor. */
export interface PipelineStage {
  id: string;
  kind: StageKind;
  /** The operative assigned to this stage, or null for unassigned. */
  operativeId: string | null;
  instruction?: string | null;
  /** For aggregate/parallel stages — the operative ids that fan out. */
  subOperatives?: string[];
  /**
   * Pass/fail routing for gate/review/test stages only.
   * When set, the compiler emits `on_pass` → `onPass` and `on_fail` → `onFail`
   * edges instead of the default `always` chain edge.
   */
  branch?: {
    onPass?: string;
    onFail?: string;
  };
}

// ---- compileToWorkflow -------------------------------------------------------

const AUTO_DONE_ID = "done";

/**
 * Compile a linear list of PipelineStages into a Workflow graph.
 *
 * Rules:
 * - If the last stage is not `done`, a `done` stage is auto-appended.
 * - Non-gate/check stages connect to the next stage with an `always` edge.
 * - Gate/check stages with a `branch` connect via `on_pass` + `on_fail` edges
 *   instead of the default chain edge (no `always` edge is emitted for them).
 * - Gate/check stages WITHOUT a `branch` fall back to a plain `always` edge
 *   to the next stage (permissive for authoring-in-progress).
 *
 * The resulting Workflow is guaranteed to pass `validateWorkflow` for any
 * well-formed linear list (unique ids, at least one stage, reachable done).
 */
export function compileToWorkflow(stages: PipelineStage[], rootGoal: string): Workflow {
  // Auto-append done if needed
  const effective: PipelineStage[] =
    stages.length > 0 && stages[stages.length - 1]!.kind === "done"
      ? stages
      : [
          ...stages,
          { id: AUTO_DONE_ID, kind: "done" as StageKind, operativeId: null },
        ];

  const nodes: WorkflowNode[] = effective.map((s) => ({
    id: s.id,
    kind: s.kind,
    operative_id: s.operativeId ?? null,
    instruction: s.instruction ?? null,
    sub_operatives: s.subOperatives ?? [],
  }));

  const edges: WorkflowEdge[] = [];
  for (let i = 0; i < effective.length - 1; i++) {
    const current = effective[i];
    const next = effective[i + 1];
    if (!current || !next) continue;

    if (isGateOrCheck(current.kind) && current.branch) {
      // Emit on_pass / on_fail edges to the named targets (no always edge)
      if (current.branch.onPass) {
        edges.push({ from: current.id, to: current.branch.onPass, when: "on_pass" });
      }
      if (current.branch.onFail) {
        edges.push({ from: current.id, to: current.branch.onFail, when: "on_fail" });
      }
      // If only one branch leg is set, still need an always edge to avoid dead end
      if (!current.branch.onPass && !current.branch.onFail) {
        edges.push({ from: current.id, to: next.id, when: "always" });
      } else if (!current.branch.onPass || !current.branch.onFail) {
        // One leg is set — add an always edge to the next stage for the missing leg
        // so the stage isn't a dead end
        edges.push({ from: current.id, to: next.id, when: "always" });
      }
    } else {
      edges.push({ from: current.id, to: next.id, when: "always" });
    }
  }

  return { schema: WORKFLOW_SCHEMA, root_goal: rootGoal, nodes, edges };
}

// ---- workflowToPipeline -----------------------------------------------------

/**
 * Inverse of `compileToWorkflow` — convert a Workflow back into a linear
 * PipelineStage array for loading templates into the linear editor.
 *
 * Best-effort: follows the `always` / `on_pass` chain from the start node.
 * Branches are captured as `branch.onPass` / `branch.onFail` on the stage.
 * Stages not reachable via the primary chain are appended at the end.
 */
export function workflowToPipeline(wf: Workflow): PipelineStage[] {
  // Build adjacency: id → { always?, on_pass?, on_fail? }
  type EdgeMap = { always?: string; on_pass?: string; on_fail?: string };
  const adj = new Map<string, EdgeMap>();
  for (const n of wf.nodes) adj.set(n.id, {});
  for (const e of wf.edges) {
    const em: EdgeMap = adj.get(e.from) ?? {};
    if (e.when === "always") em.always = e.to;
    else if (e.when === "on_pass") em.on_pass = e.to;
    else if (e.when === "on_fail") em.on_fail = e.to;
    adj.set(e.from, em);
  }

  // Find start node (no incoming edges)
  const hasIncoming = new Set(wf.edges.map((e) => e.to));
  const nodeMap = new Map(wf.nodes.map((n) => [n.id, n]));
  const startNodes = wf.nodes.filter((n) => !hasIncoming.has(n.id));
  if (startNodes.length === 0) return [];

  // Walk the primary chain (always / on_pass preferred)
  const visited = new Set<string>();
  const ordered: string[] = [];
  let current: string | undefined = startNodes[0]!.id;
  while (current && !visited.has(current)) {
    visited.add(current);
    ordered.push(current);
    const em: EdgeMap = adj.get(current) ?? {};
    current = em.always ?? em.on_pass;
  }

  // Append any unvisited nodes (shouldn't happen for a linear list, but be safe)
  for (const n of wf.nodes) {
    if (!visited.has(n.id)) ordered.push(n.id);
  }

  return ordered.map((id): PipelineStage => {
    const node = nodeMap.get(id)!;
    const em: EdgeMap = adj.get(id) ?? {};
    const hasBranch = em.on_pass !== undefined || em.on_fail !== undefined;
    return {
      id: node.id,
      kind: node.kind,
      operativeId: node.operative_id ?? null,
      instruction: node.instruction ?? null,
      subOperatives: node.sub_operatives?.length ? node.sub_operatives : undefined,
      branch: hasBranch
        ? { onPass: em.on_pass, onFail: em.on_fail }
        : undefined,
    };
  });
}

// ---- pure array helpers ------------------------------------------------------

/**
 * Add a stage to the array, optionally at a specific index (appends if omitted).
 * Pure — returns a new array.
 */
export function addStage(
  stages: PipelineStage[],
  stage: PipelineStage,
  atIndex?: number,
): PipelineStage[] {
  if (atIndex === undefined || atIndex >= stages.length) {
    return [...stages, stage];
  }
  const result = [...stages];
  result.splice(atIndex, 0, stage);
  return result;
}

/**
 * Remove a stage by id. Pure — returns a new array.
 * If the id is not found the original contents are returned (same length).
 */
export function removeStage(stages: PipelineStage[], id: string): PipelineStage[] {
  return stages.filter((s) => s.id !== id);
}

/**
 * Move a stage from one index to another. Pure — returns a new array.
 * Clamps indices to valid bounds.
 */
export function moveStage(stages: PipelineStage[], from: number, to: number): PipelineStage[] {
  if (from === to || stages.length === 0) return [...stages];
  const result = [...stages];
  const [item] = result.splice(from, 1) as [PipelineStage];
  result.splice(to, 0, item);
  return result;
}

// Re-export validateWorkflow so callers can validate without importing workflow.ts directly.
export { validateWorkflow };
