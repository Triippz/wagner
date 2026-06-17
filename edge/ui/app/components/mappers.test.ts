import { describe, it, expect } from "vitest";
import type { VaultNodeDto, VaultEdgeDto } from "../bridge";
import { toFlowNodes, toFlowEdges } from "./mappers";

const nodes: VaultNodeDto[] = [
  { uid: "aaa", title: "Alpha", tier: "core", lifecycle: "active" },
  { uid: "bbb", title: "Beta", tier: "supporting", lifecycle: "draft" },
  { uid: "ccc", title: "", tier: "", lifecycle: "" },
];

const edges: VaultEdgeDto[] = [
  { sourceUid: "aaa", targetUid: "bbb", relType: "references" },
  { sourceUid: "bbb", targetUid: "ccc", relType: "wikilink" },
];

describe("toFlowNodes", () => {
  it("maps each node to a React Flow Node", () => {
    const result = toFlowNodes(nodes);
    expect(result).toHaveLength(3);
    const n0 = result[0]!;
    expect(n0.id).toBe("aaa");
    expect(n0.data.uid).toBe("aaa");
    expect(n0.data.title).toBe("Alpha");
    expect(n0.type).toBe("vaultNode");
  });

  it("positions nodes in a grid: x = (i%10)*180, y = floor(i/10)*120", () => {
    const result = toFlowNodes(nodes);
    expect(result[0]!.position).toEqual({ x: 0, y: 0 });
    expect(result[1]!.position).toEqual({ x: 180, y: 0 });
    expect(result[2]!.position).toEqual({ x: 360, y: 0 });
  });

  it("marks focusedId node with focused=true, others false", () => {
    const result = toFlowNodes(nodes, "bbb");
    expect(result[0]!.data.focused).toBe(false);
    expect(result[1]!.data.focused).toBe(true);
    expect(result[2]!.data.focused).toBe(false);
  });

  it("marks no nodes focused when focusedId is undefined", () => {
    const result = toFlowNodes(nodes);
    for (const n of result) {
      expect(n.data.focused).toBe(false);
    }
  });
});

describe("toFlowEdges", () => {
  it("maps each edge to a React Flow Edge", () => {
    const result = toFlowEdges(edges);
    expect(result).toHaveLength(2);
    const e0 = result[0]!;
    expect(e0.id).toBe("aaa->bbb:references");
    expect(e0.source).toBe("aaa");
    expect(e0.target).toBe("bbb");
    expect(e0.label).toBe("references");
  });

  it("produces stable ids from source+target+relType", () => {
    const result = toFlowEdges(edges);
    expect(result[1]!.id).toBe("bbb->ccc:wikilink");
  });
});
