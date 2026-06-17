// Hub E2E: boots a real Deno.serve on a random port; exercises endpoints over
// genuine HTTP. OIDC is stubbed with a local JWKS so no network leaves the box.

import { assertEquals } from "@std/assert";
import { exportJWK, generateKeyPair, type JSONWebKeySet, SignJWT } from "jose";
import { createApp } from "../../src/app.ts";

// ── OIDC stub ────────────────────────────────────────────────────────────────

const ISSUER = "https://accounts.google.com";
const CLIENT_ID = "wagner-hub-e2e";

const { publicKey, privateKey } = await generateKeyPair("RS256");
const jwk = await exportJWK(publicKey);
jwk.kid = "e2e-k1";
jwk.alg = "RS256";
const jwks: JSONWebKeySet = { keys: [jwk] };

async function mintToken(claims: Record<string, unknown>): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return new SignJWT(claims)
    .setProtectedHeader({ alg: "RS256", kid: "e2e-k1" })
    .setIssuer(ISSUER)
    .setAudience(CLIENT_ID)
    .setIssuedAt(now)
    .setExpirationTime(now + 3600)
    .setSubject(claims.sub as string)
    .sign(privateKey);
}

// ── Server lifecycle ──────────────────────────────────────────────────────────

interface ServerHandle {
  base: string;
  shutdown: () => Promise<void>;
}

async function startServer(): Promise<ServerHandle> {
  const { fetch: handler } = createApp({
    oidc: {
      allowedIssuers: [ISSUER],
      clientId: CLIENT_ID,
      allowedDomains: ["adyton.io"],
      jwksResolver: () => Promise.resolve(jwks),
    },
  });

  // port 0 → OS assigns a free port (hermetic, no hard-coded port)
  let resolve: (v: { hostname: string; port: number }) => void;
  const ready = new Promise<{ hostname: string; port: number }>((r) => { resolve = r; });

  const server = Deno.serve(
    { port: 0, hostname: "127.0.0.1", onListen: resolve! },
    handler,
  );

  const { hostname, port } = await ready;
  return {
    base: `http://${hostname}:${port}`,
    shutdown: () => server.shutdown(),
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

Deno.test("E2E: GET /health returns 200 and status ok (no auth)", async () => {
  const srv = await startServer();
  try {
    const res = await fetch(`${srv.base}/health`);
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body.status, "ok");
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: GET /v1/whoami without token → 401", async () => {
  const srv = await startServer();
  try {
    const res = await fetch(`${srv.base}/v1/whoami`);
    assertEquals(res.status, 401);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: GET /v1/whoami with valid employee token → 200", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "e2e-op-1", email: "mark@adyton.io", email_verified: true });
    const res = await fetch(`${srv.base}/v1/whoami`, {
      headers: { authorization: `Bearer ${tok}` },
    });
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body.operator.operatorId, "e2e-op-1");
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: GET /v1/whoami with non-employee token → 403", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "outsider", email: "x@gmail.com", email_verified: true });
    const res = await fetch(`${srv.base}/v1/whoami`, {
      headers: { authorization: `Bearer ${tok}` },
    });
    assertEquals(res.status, 403);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: discovery register → resolve → disarm round-trip", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "e2e-op-2", email: "alice@adyton.io", email_verified: true });
    const headers = { "content-type": "application/json", authorization: `Bearer ${tok}` };

    // arm
    const regRes = await fetch(`${srv.base}/v1/discovery/register`, {
      method: "POST",
      headers,
      body: JSON.stringify({ node_id: "node-abc", ticket: "ticket-xyz", ttl_ms: 30000 }),
    });
    assertEquals(regRes.status, 200);
    const regBody = await regRes.json();
    assertEquals(regBody.status, "armed");
    assertEquals(regBody.operator_id, "e2e-op-2");

    // owner can resolve their own entry
    const resolveRes = await fetch(`${srv.base}/v1/discovery/resolve`, {
      method: "POST",
      headers,
      body: JSON.stringify({ operator_id: "e2e-op-2" }),
    });
    assertEquals(resolveRes.status, 200);
    const resolveBody = await resolveRes.json();
    assertEquals(resolveBody.node_id, "node-abc");
    assertEquals(resolveBody.ticket, "ticket-xyz");

    // re-arm to reset (also tests idempotent arm)
    const rearm = await fetch(`${srv.base}/v1/discovery/register`, {
      method: "POST",
      headers,
      body: JSON.stringify({ node_id: "node-abc", ticket: "ticket-xyz" }),
    });
    assertEquals(rearm.status, 200);
    await rearm.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: resolve returns 404 for never-armed operator", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "e2e-op-3", email: "bob@adyton.io", email_verified: true });
    const res = await fetch(`${srv.base}/v1/discovery/resolve`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${tok}` },
      body: JSON.stringify({ operator_id: "e2e-op-3" }),
    });
    assertEquals(res.status, 404);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: register with invalid body → 400", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "e2e-op-4", email: "carol@adyton.io", email_verified: true });
    const res = await fetch(`${srv.base}/v1/discovery/register`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${tok}` },
      body: JSON.stringify({ node_id: 123 }), // missing ticket, bad node_id type
    });
    assertEquals(res.status, 400);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});
