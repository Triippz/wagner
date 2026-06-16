// T008 — Remote-identity discovery contract (FR-200/201/212, EC-003). RED first.
//
// Discovery is the only hub touchpoint a remote peer hits before a channel
// opens. This contract asserts:
//   - register (arm) and resolve (attach lookup) are OIDC-gated: no / expired /
//     wrong-audience token → refused BEFORE anything is registered or resolved;
//   - a valid org token registers an armed host;
//   - resolve is owner-only: the owning operator resolves their host; a
//     different verified operator gets 404 (no cross-operator discovery);
//   - resolve of a never-armed / disarmed host is 404.
// T013 implements POST /v1/discovery/register + /resolve to make this pass.

import { assertEquals } from "@std/assert";
import { exportJWK, generateKeyPair, type JSONWebKeySet, SignJWT } from "jose";
import { createApp } from "../../src/app.ts";

const ISSUER = "https://accounts.google.com";
const CLIENT_ID = "wagner-hub-client";

const { publicKey, privateKey } = await generateKeyPair("RS256");
const jwk = await exportJWK(publicKey);
jwk.kid = "k1";
jwk.alg = "RS256";
const jwks: JSONWebKeySet = { keys: [jwk] };

async function token(
  sub: string,
  opts: { email?: string; aud?: string; expOffset?: number } = {},
): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return await new SignJWT({ email: opts.email ?? `${sub}@adyton.io`, email_verified: true })
    .setProtectedHeader({ alg: "RS256", kid: "k1" })
    .setIssuer(ISSUER)
    .setAudience(opts.aud ?? CLIENT_ID)
    .setIssuedAt(now)
    .setExpirationTime(now + (opts.expOffset ?? 3600))
    .setSubject(sub)
    .sign(privateKey);
}

function app() {
  return createApp({
    oidc: {
      allowedIssuers: [ISSUER],
      clientId: CLIENT_ID,
      allowedDomains: ["adyton.io"],
      jwksResolver: () => Promise.resolve(jwks),
    },
  });
}

const REG_BODY = { node_id: "n0deADDR", ticket: "tkt-1", ttl_ms: 60_000 };

function post(path: string, body: unknown, bearer?: string): Request {
  return new Request(`http://h${path}`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(bearer ? { authorization: `Bearer ${bearer}` } : {}),
    },
    body: JSON.stringify(body),
  });
}

Deno.test("register without a token → 401, nothing armed", async () => {
  const a = app();
  const res = await a.fetch(post("/v1/discovery/register", REG_BODY));
  assertEquals(res.status, 401);
  assertEquals(a.registry.size(), 0);
});

Deno.test("register with an expired token → 401, nothing armed", async () => {
  const a = app();
  const t = await token("op-1", { expOffset: -60 });
  const res = await a.fetch(post("/v1/discovery/register", REG_BODY, t));
  assertEquals(res.status, 401);
  assertEquals(a.registry.size(), 0);
});

Deno.test("register with a wrong-audience token → 401, nothing armed", async () => {
  const a = app();
  const t = await token("op-1", { aud: "another-app" });
  const res = await a.fetch(post("/v1/discovery/register", REG_BODY, t));
  assertEquals(res.status, 401);
  assertEquals(a.registry.size(), 0);
});

Deno.test("valid org token registers an armed host", async () => {
  const a = app();
  const t = await token("op-1");
  const res = await a.fetch(post("/v1/discovery/register", REG_BODY, t));
  assertEquals(res.status, 200);
  assertEquals(a.registry.size(), 1);
});

Deno.test("owner resolves their own armed host", async () => {
  const a = app();
  const t = await token("op-1");
  await a.fetch(post("/v1/discovery/register", REG_BODY, t));
  const res = await a.fetch(post("/v1/discovery/resolve", { operator_id: "op-1" }, t));
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body.node_id, "n0deADDR");
  assertEquals(body.ticket, "tkt-1");
});

Deno.test("a different verified operator cannot resolve another's host → 404", async () => {
  const a = app();
  const owner = await token("op-1");
  await a.fetch(post("/v1/discovery/register", REG_BODY, owner));
  const other = await token("op-2");
  const res = await a.fetch(post("/v1/discovery/resolve", { operator_id: "op-1" }, other));
  assertEquals(res.status, 404);
});

Deno.test("resolve of a never-armed host → 404", async () => {
  const a = app();
  const t = await token("ghost");
  const res = await a.fetch(post("/v1/discovery/resolve", { operator_id: "ghost" }, t));
  assertEquals(res.status, 404);
});
