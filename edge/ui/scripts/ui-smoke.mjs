// Headless UI smoke test for the Wagner Edge console.
//
// The React surface is transport-blind, so we drive it WITHOUT the native Tauri
// shell: `main.tsx`'s `?mock` seam swaps in a fake transport exposing
// `window.__wagner.push(channel, payload)`. This script spawns the vite dev
// server, drives the UI through composer -> running console -> operative
// inspector -> permission prompt via Playwright, screenshots each, and fails on
// any console error or missing assertion. It is the autonomous browser-level
// counterpart to a live `make platform-edge` launch (which covers the native
// shell — window/tray/real IPC — that headless Playwright cannot).
//
// Run: `npm --prefix platform/edge/ui run test:ui`  (or `make platform-edge-ui`).

import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url))); // edge/ui
const OUT = process.env.UI_SHOTS_DIR || join(ROOT, ".ui-shots");
const PORT = 1420;
const BASE = `http://localhost:${PORT}/?mock`;

mkdirSync(OUT, { recursive: true });

function waitForServer(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const tick = async () => {
      try {
        const r = await fetch(url);
        if (r.ok) return resolve();
      } catch {
        /* not up yet */
      }
      if (Date.now() > deadline) return reject(new Error("vite dev server never came up"));
      setTimeout(tick, 250);
    };
    tick();
  });
}

const vite = spawn("npm", ["run", "dev", "--", "--port", String(PORT), "--strictPort"], {
  cwd: ROOT,
  stdio: "ignore",
});

let browser;
let failed = false;
try {
  await waitForServer(`http://localhost:${PORT}/`, 30_000);

  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 832 } });
  const errors = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("pageerror", (e) => errors.push(`PAGEERROR: ${e.message}`));

  await page.goto(BASE, { waitUntil: "networkidle" });

  // 1) Composer (no run yet).
  await page.waitForSelector("text=Launch a run", { timeout: 8000 });
  await page.screenshot({ path: join(OUT, "1-composer.png") });

  // 2) A running run + two operatives + subtasks -> console view.
  const ts = new Date().toISOString();
  await page.evaluate((now) => {
    const w = window.__wagner;
    w.push("run", {
      schema: "wagner-run.v1", run_id: "01J0RUN",
      goal: "Add a /healthz endpoint with a test and wire it into the router.",
      status: "running", phase: "dispatching", iteration: 2,
      guardrails: { blocked_timeout_secs: 120, iterations_used: 2, cost: { mode: "cli_usage", budget: 5, used: 1.24 } },
      subtasks: [
        { id: "s1", agent_id: "cipher", prompt: "Write the failing /healthz test", state: "done", result_summary: "test added" },
        { id: "s2", agent_id: "vex", prompt: "Implement the handler + wire the router", state: "running" },
      ],
    });
    const ev = (id, name, faction, activity, district, state, message) =>
      w.push("event", {
        schema: "wagner-event.v1", event_id: id + now, run_id: "01J0RUN", operative_id: id,
        operative_name: name, faction, activity, district, state, message,
        handoff_target_operative_id: null, ts: now,
      });
    ev("cipher", "Cipher", "architects", "plan", "oracle", "thinking", "Decomposing the goal into two subtasks");
    ev("vex", "Vex", "forgers", "edit", "stacks", "working", "Editing src/router.rs to register /healthz");
  }, ts);

  await page.waitForSelector("text=Running", { timeout: 8000 });
  await page.waitForSelector("text=Vex", { timeout: 8000 });

  // 2b) Select an operative -> transcript inspector.
  await page.click("text=Vex");
  await page.waitForSelector("text=Editing src/router.rs", { timeout: 8000 });
  await page.waitForTimeout(450); // let the entrance stagger settle for the shot
  await page.screenshot({ path: join(OUT, "2-console.png") });

  // 3) A permission transmission -> needs-you prompt.
  await page.evaluate((now) => {
    window.__wagner.push("transmission", {
      schema: "transmission.v1", id: "t1", subtask_id: "s2", kind: "permission",
      prompt: "Allow Vex to run `git push origin feat/healthz`?",
      options: [{ id: "allow", label: "Approve" }, { id: "deny", label: "Deny" }],
      raised_at: now, state: "open",
    });
  }, ts);
  await page.waitForSelector("text=Permission requested", { timeout: 8000 });
  await page.waitForSelector("text=Needs you", { timeout: 8000 });
  await page.waitForTimeout(450);
  await page.screenshot({ path: join(OUT, "3-permission.png") });

  // 4) Vault graph smoke — dismiss the open transmission first so the Vault tab is enabled.
  await page.evaluate(() => {
    // Simulate answering the transmission so needsYou clears (mock: no real IPC, so
    // we push an updated transmission with state=closed to close the prompt).
    window.__wagner.push("transmission", {
      schema: "transmission.v1", id: "t1", subtask_id: "s2", kind: "permission",
      prompt: "Allow Vex to run `git push origin feat/healthz`?",
      options: [{ id: "allow", label: "Approve" }, { id: "deny", label: "Deny" }],
      raised_at: new Date().toISOString(), state: "closed",
    });
  });

  // Click Vault tab first so VaultPanel mounts and registers its .on() listener.
  await page.click("button:has-text('Vault'):not([disabled])");
  // Wait for the loading state to confirm VaultPanel has mounted.
  await page.waitForSelector("text=Loading vault", { timeout: 5000 });

  // Now push mock vault graph data — VaultPanel's side-channel listener is live.
  await page.evaluate(() => {
    window.__wagner.push("vault_graph_result", {
      nodes: [
        { uid: "n1", title: "Alpha Node", tier: "core", lifecycle: "active" },
        { uid: "n2", title: "Beta Node", tier: "supporting", lifecycle: "draft" },
      ],
      edges: [{ sourceUid: "n1", targetUid: "n2", relType: "references" }],
    });
  });

  await page.waitForSelector(".vault-node", { timeout: 8000 });
  await page.screenshot({ path: join(OUT, "4-vault-graph.png") });

  // Click a vault-node. React Flow registers click handlers on its pane; clicking
  // the node wrapper (or the vault-node div inside it) triggers onNodeClick.
  // We use the vault-node div directly — its own onClick handler also fires toggleFocus.
  const vaultNode = page.locator(".vault-node").first();
  await vaultNode.click();
  await page.waitForSelector('.vault-node[data-focused="true"]', { timeout: 5000 });
  await page.screenshot({ path: join(OUT, "5-vault-node-focused.png") });

  // Click the same vault-node again — data-focused should clear.
  await vaultNode.click();
  await page.waitForFunction(
    () => document.querySelectorAll('.vault-node[data-focused="true"]').length === 0,
    { timeout: 5000 },
  );
  await page.screenshot({ path: join(OUT, "6-vault-node-unfocused.png") });

  if (errors.length) {
    failed = true;
    console.error(`UI smoke FAILED — ${errors.length} console error(s):`);
    for (const e of errors) console.error("  " + e);
  } else {
    console.log(`UI smoke PASSED — composer, console, inspector, permission, vault graph all render; 0 console errors. Shots in ${OUT}`);
  }
} catch (e) {
  failed = true;
  console.error("UI smoke ERROR:", e.message);
} finally {
  if (browser) await browser.close();
  vite.kill("SIGTERM");
}

process.exit(failed ? 1 : 0);
