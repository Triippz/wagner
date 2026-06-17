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
function makeMockTransport(): EventStreamTransport {
  let sink: ((e: TransportEvent) => void) | null = null;
  (window as unknown as { __wagner: unknown }).__wagner = {
    push: (channel: string, payload: unknown) => sink?.({ channel, payload }),
  };
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
