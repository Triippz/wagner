#!/usr/bin/env node
// The Construct — US2 permission gate (MCP stdio server).
//
// Claude spawns this as its `--permission-prompt-tool` server. It runs in a
// DIFFERENT process than the Construct app, so on each `can_use_tool` call it
// forwards the payload to the app's loopback permission server (CONSTRUCT_GATE_URL)
// and returns the engineer's decision. If the app is unreachable it FAILS SAFE
// (deny) rather than hanging Claude or silently allowing.
//
// Zero dependencies: a minimal newline-delimited JSON-RPC 2.0 stdio server.
// stdout is the MCP wire protocol — all logging goes to stderr.

import { request as httpRequest } from "node:http";

const GATE_URL = process.env.CONSTRUCT_GATE_URL || "";
// Per-run secret the app's permission server requires on every request, so only
// this gate (not any other local process) can approve tool use (M2).
const GATE_TOKEN = process.env.CONSTRUCT_GATE_TOKEN || "";
const log = (m) => process.stderr.write(`[wagner-gate] ${m}\n`);
const send = (msg) => process.stdout.write(JSON.stringify(msg) + "\n");

function denyResult(message) {
  return {
    content: [{ type: "text", text: JSON.stringify({ behavior: "deny", message }) }],
  };
}

// POST the permission payload to the app and resolve with its decision JSON.
function askApp(payload) {
  return new Promise((resolve) => {
    if (!GATE_URL) return resolve({ behavior: "deny", message: "gate URL unset" });
    let url;
    try { url = new URL(GATE_URL); } catch { return resolve({ behavior: "deny", message: "bad gate URL" }); }
    const body = Buffer.from(JSON.stringify(payload));
    const req = httpRequest(
      {
        hostname: url.hostname,
        port: url.port,
        path: url.pathname,
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": body.length,
          "X-Gate-Token": GATE_TOKEN,
        },
      },
      (res) => {
        let data = "";
        res.setEncoding("utf8");
        res.on("data", (c) => (data += c));
        res.on("end", () => {
          try { resolve(JSON.parse(data)); }
          catch { resolve({ behavior: "deny", message: "bad gate response" }); }
        });
      }
    );
    // No client timeout: the engineer may take their time. The app's
    // blocked-too-long guardrail bounds the wait and the run can be aborted.
    req.on("error", (e) => resolve({ behavior: "deny", message: `gate unreachable: ${e.message}` }));
    req.write(body);
    req.end();
  });
}

let buf = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buf += chunk;
  let idx;
  while ((idx = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (line) handle(line);
  }
});

async function handle(line) {
  let msg;
  try { msg = JSON.parse(line); } catch { return; }
  const { id, method, params } = msg;
  if (method === "initialize") {
    send({ jsonrpc: "2.0", id, result: {
      protocolVersion: params?.protocolVersion || "2024-11-05",
      capabilities: { tools: {} },
      serverInfo: { name: "gate", version: "1.0.0" },
    }});
  } else if (method === "notifications/initialized") {
    // notifications get no response
  } else if (method === "tools/list") {
    send({ jsonrpc: "2.0", id, result: { tools: [{
      name: "approve",
      description: "Route a tool-use permission request to The Construct operator.",
      inputSchema: {
        type: "object",
        properties: { tool_name: { type: "string" }, input: { type: "object" } },
      },
    }]}});
  } else if (method === "tools/call") {
    const args = params?.arguments ?? {};
    log(`permission: ${args.tool_name}`);
    const decision = await askApp({
      tool_name: args.tool_name,
      input: args.input ?? args.tool_input ?? {},
      tool_use_id: args.tool_use_id ?? null,
    });
    send({ jsonrpc: "2.0", id, result: {
      content: [{ type: "text", text: JSON.stringify(decision) }],
    }});
  } else if (id !== undefined) {
    send({ jsonrpc: "2.0", id, error: { code: -32601, message: `method not found: ${method}` } });
  }
}

log(GATE_URL ? `started → ${GATE_URL}` : "started (no gate URL — will deny)");
