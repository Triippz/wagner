import { useEffect, useState } from "react";
import type { Surface } from "../../surfaces/surface";
import type { VaultGraphDto } from "../bridge";
import { cmd } from "../bridge";
import { VaultGraph } from "./VaultGraph";

interface Props {
  surface: Surface;
  projectDir: string;
}

// In ?mock mode the smoke test pushes via window.__wagner.push("vault_graph_result", graph).
// In real mode we call cmd.vaultGraph() on mount.
const useMock =
  typeof window !== "undefined" &&
  new URLSearchParams(window.location.search).has("mock");

type MockHandle = { on: (ch: string, cb: (p: unknown) => void) => () => void };

export function VaultPanel({ surface: _surface, projectDir }: Props) {
  const [graph, setGraph] = useState<VaultGraphDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (useMock) {
      const handle = (window as unknown as { __wagner?: MockHandle }).__wagner;
      if (!handle?.on) return;
      return handle.on("vault_graph_result", (payload) => {
        setGraph(payload as VaultGraphDto);
      });
    }
    cmd
      .vaultGraph(projectDir)
      .then(setGraph)
      .catch((e: unknown) => setError(String(e)));
  }, [projectDir]);

  if (error) return <div className="vault-panel-error">{error}</div>;
  if (!graph) return <div className="vault-panel-loading">Loading vault…</div>;

  return (
    <div className="vault-panel">
      <VaultGraph graph={graph} />
    </div>
  );
}
