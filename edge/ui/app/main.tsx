import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { EventStreamTransport, TransportEvent } from "@wagner/shared/transport";
import { createTransport } from "../transport";
import { createSurface } from "../surfaces/surface";
import { tauriBridge } from "./bridge";
import { App } from "./App";
import "./styles.css";

// Dev/e2e seam: with `?mock`, fold a Playwright-driven event stream instead of
// the Tauri bridge, so the browser UI is testable headlessly (no native shell).
// `window.__wagner.push(channel, payload)` feeds the same surface the real
// transport would. DEV-only and query-gated — never reaches a production bundle.
// Side-channel listeners (e.g. VaultPanel) subscribe via `window.__wagner.on`.
type WagnerMockHandle = {
  push: (channel: string, payload: unknown) => void;
  on: (channel: string, cb: (payload: unknown) => void) => () => void;
};

function makeMockTransport(): EventStreamTransport {
  let sink: ((e: TransportEvent) => void) | null = null;
  const sideListeners = new Map<string, Set<(p: unknown) => void>>();
  const handle: WagnerMockHandle = {
    push: (channel: string, payload: unknown) => {
      // Feed the surface subscriber (known channels: event/run/transmission).
      sink?.({ channel, payload });
      // Also route to side-channel listeners (e.g. vault_graph_result).
      sideListeners.get(channel)?.forEach((cb) => cb(payload));
    },
    on: (channel: string, cb: (payload: unknown) => void) => {
      if (!sideListeners.has(channel)) sideListeners.set(channel, new Set());
      sideListeners.get(channel)!.add(cb);
      return () => sideListeners.get(channel)?.delete(cb);
    },
  };
  (window as unknown as { __wagner: WagnerMockHandle }).__wagner = handle;
  return {
    subscribe(onEvent) {
      sink = onEvent;
      return () => {
        sink = null;
      };
    },
    send: async () => {},
  };
}

const useMock =
  import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock");

// Desktop path: fold the host event stream over the in-process IPC transport.
// `hasTauri: true` is asserted (not detected) because this entry only ships
// inside the Tauri webview; the bridge's listen/invoke no-op gracefully if the
// runtime is ever absent (e.g. plain `vite preview`).
const transport = useMock
  ? makeMockTransport()
  : createTransport({ hasTauri: true }, { bridge: tauriBridge });
const surface = createSurface(transport);

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("root element missing");
createRoot(rootEl).render(
  <StrictMode>
    <App surface={surface} />
  </StrictMode>,
);
