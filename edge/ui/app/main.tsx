import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createTransport } from "../transport";
import { createSurface } from "../surfaces/surface";
import { tauriBridge } from "./bridge";
import { App } from "./App";
import "./styles.css";

// Desktop path: fold the host event stream over the in-process IPC transport.
// `hasTauri: true` is asserted (not detected) because this entry only ships
// inside the Tauri webview; the bridge's listen/invoke no-op gracefully if the
// runtime is ever absent (e.g. plain `vite preview`).
const transport = createTransport({ hasTauri: true }, { bridge: tauriBridge });
const surface = createSurface(transport);

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("root element missing");
createRoot(rootEl).render(
  <StrictMode>
    <App surface={surface} />
  </StrictMode>,
);
