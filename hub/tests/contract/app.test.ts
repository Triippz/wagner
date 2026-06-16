// T000c — Hub app contract: health is open; /v1 is OIDC-gated. Test FIRST.

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

async function token(claims: Record<string, unknown>, expOffset = 3600): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return await new SignJWT(claims)
    .setProtectedHeader({ alg: "RS256", kid: "k1" })
    .setIssuer(ISSUER)
    .setAudience(CLIENT_ID)
    .setIssuedAt(now)
    .setExpirationTime(now + expOffset)
    .setSubject(claims.sub as string)
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

Deno.test("GET /health is open and returns ok", async () => {
  const res = await app().fetch(new Request("http://h/health"));
  assertEquals(res.status, 200);
  assertEquals((await res.json()).status, "ok");
});

Deno.test("GET /v1/whoami without a token → 401", async () => {
  const res = await app().fetch(new Request("http://h/v1/whoami"));
  assertEquals(res.status, 401);
});

Deno.test("GET /v1/whoami with a valid employee token → 200 + operator", async () => {
  const t = await token({ sub: "op-1", email: "mark@adyton.io", email_verified: true });
  const res = await app().fetch(
    new Request("http://h/v1/whoami", { headers: { authorization: `Bearer ${t}` } }),
  );
  assertEquals(res.status, 200);
  assertEquals((await res.json()).operator.operatorId, "op-1");
});

Deno.test("GET /v1/whoami with a non-employee token → 403", async () => {
  const t = await token({ sub: "op-x", email: "x@gmail.com", email_verified: true });
  const res = await app().fetch(
    new Request("http://h/v1/whoami", { headers: { authorization: `Bearer ${t}` } }),
  );
  assertEquals(res.status, 403);
});
