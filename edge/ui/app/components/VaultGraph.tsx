import { useState, useCallback } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  type NodeProps,
  type Node,
  type NodeMouseHandler,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { VaultGraphDto } from "../bridge";
import { toFlowNodes, toFlowEdges, type VaultNodeData } from "./mappers";

// Custom vault node — carries data-tier for CSS tier-color and
// data-focused for Playwright selectors (.vault-node[data-focused="true"]).
// onClick is also wired directly so Playwright force-click works in smoke tests.
function VaultNodeCard({ data }: NodeProps<Node<VaultNodeData>>) {
  return (
    <div
      className="vault-node"
      data-tier={data.tier || "unknown"}
      data-focused={String(data.focused)}
      onClick={(e) => {
        e.stopPropagation();
        const cb = data._onNodeClick as ((uid: string) => void) | undefined;
        cb?.(data.uid);
      }}
    >
      {data.title || String(data.uid).slice(0, 8)}
    </div>
  );
}

const nodeTypes = { vaultNode: VaultNodeCard };

export function VaultGraph({ graph }: { graph: VaultGraphDto }) {
  const [focusedId, setFocusedId] = useState<string | null>(null);

  const toggleFocus = useCallback((uid: string) => {
    setFocusedId((prev) => (prev === uid ? null : uid));
  }, []);

  const flowNodes = toFlowNodes(graph.nodes, focusedId ?? undefined, toggleFocus);
  const flowEdges = toFlowEdges(graph.edges);

  const onNodeClick: NodeMouseHandler = useCallback((_evt, node) => {
    setFocusedId((prev) => (prev === node.id ? null : node.id));
  }, []);

  return (
    <div style={{ width: "100%", height: "100%" }}>
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
        fitView
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
