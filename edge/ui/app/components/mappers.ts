// Pure mapper functions: VaultGraphDto → React Flow nodes/edges.
// No hooks — safe to call outside React components and vitest-testable.

import type { Node, Edge } from "@xyflow/react";
import type { VaultNodeDto, VaultEdgeDto } from "../bridge";

export type VaultNodeData = VaultNodeDto & {
  focused: boolean;
  _onNodeClick?: (uid: string) => void;
  [key: string]: unknown;
};

/** Map vault nodes to React Flow nodes in a simple grid layout.
 *  @param toggleFocus optional callback threaded into node data for direct-click smoke tests.
 */
export function toFlowNodes(
  nodes: VaultNodeDto[],
  focusedId?: string,
  toggleFocus?: (uid: string) => void,
): Node<VaultNodeData>[] {
  return nodes.map((n, i) => ({
    id: n.uid,
    type: "vaultNode",
    position: { x: (i % 10) * 180, y: Math.floor(i / 10) * 120 },
    data: { ...n, focused: n.uid === focusedId, _onNodeClick: toggleFocus },
  }));
}

/** Map vault edges to React Flow edges with stable composite ids. */
export function toFlowEdges(edges: VaultEdgeDto[]): Edge[] {
  return edges.map((e) => ({
    id: `${e.sourceUid}->${e.targetUid}:${e.relType}`,
    source: e.sourceUid,
    target: e.targetUid,
    label: e.relType,
  }));
}
