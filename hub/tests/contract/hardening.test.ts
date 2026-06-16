// T046 — Security hardening confirmations (D-SEC-2, Article X, SC-006/203).
//
// Cross-cutting checks: the hub validates channel ingress (a malformed body is
// rejected before any state change), takes ALL config by injection (no inlined
// secrets / issuer config), and the discovery surface offers no self-arm path.

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

async function token(sub: string): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return await new SignJWT({ email: `${sub}@adyton.io`, email_verified: true })
    .setProtectedHeader({ alg: "RS256", kid: "k1" })
    .setIssuer(ISSUER).setAudience(CLIENT_ID)
    .setIssuedAt(now).setExpirationTime(now + 3600).setSubject(sub)
    .sign(privateKey);
}

function app() {
  return createApp({
    oidc: { allowedIssuers: [ISSUER], clientId: CLIENT_ID, allowedDomains: ["adyton.io"], jwksResolver: () => Promise.resolve(jwks) },
  });
}

function post(path: string, body: unknown, bearer: string): Request {
  return new Request(`http://h${path}`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${bearer}` },
    body: JSON.stringify(body),
  });
}

Deno.test("malformed register ingress is rejected (400), nothing armed", async () => {
  const a = app();
  const t = await token("op-1");
  // Missing node_id/ticket.
  const res = await a.fetch(post("/v1/discovery/register", { bogus: true }, t));
  assertEquals(res.status, 400);
  assertEquals(a.registry.size(), 0);
});

Deno.test("malformed resolve ingress is rejected (400)", async () => {
  const a = app();
  const t = await token("op-1");
  const res = await a.fetch(post("/v1/discovery/resolve", {}, t));
  assertEquals(res.status, 400);
});

Deno.test("there is no self-arm path: register arms only the VERIFIED caller", async () => {
  const a = app();
  const t = await token("op-1");
  // Even if the body tries to name another operator, arming uses the token sub.
  await a.fetch(post("/v1/discovery/register", { node_id: "n", ticket: "tk", operator_id: "op-OTHER" }, t));
  // op-OTHER is NOT armed; only op-1 (the verified caller) is.
  const other = await token("op-OTHER");
  const stolen = await a.fetch(post("/v1/discovery/resolve", { operator_id: "op-OTHER" }, other));
  assertEquals(stolen.status, 404, "no host was armed under the body-supplied operator");
  const ownResolve = await a.fetch(post("/v1/discovery/resolve", { operator_id: "op-1" }, t));
  assertEquals(ownResolve.status, 200, "only the verified caller's host is armed");
});
