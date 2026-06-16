// T000c — OIDC ID-token verification (ADR-0002). Test FIRST.
//
// The hub stores no credentials: it validates an IdP-issued ID token
// (signature against the issuer JWKS, issuer ∈ allowed, audience == client_id,
// not expired) and gates access by verified email domain / IdP group.
// `operator_id` is the IdP subject. These tests sign tokens locally against an
// in-process JWKS so no network/IdP is touched (D-TEST-4 spirit).

import { assertEquals, assertRejects } from "@std/assert";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { OidcError, verifyIdToken } from "../../src/auth/oidc.ts";

const ISSUER = "https://accounts.google.com";
const CLIENT_ID = "wagner-hub-client";
const ALLOWED_DOMAINS = ["adyton.io"];

// One keypair stands in for the IdP signing key; its public half is the JWKS.
const { publicKey, privateKey } = await generateKeyPair("RS256");
const jwk = await exportJWK(publicKey);
jwk.kid = "test-key-1";
jwk.alg = "RS256";
const JWKS = { keys: [jwk] };

// A local JWKS resolver injected into the verifier — no network.
const localJwks = () => Promise.resolve(JWKS);

function makeToken(
  claims: Record<string, unknown>,
  opts: { aud?: string; iss?: string; expSecondsFromNow?: number } = {},
): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return new SignJWT(claims)
    .setProtectedHeader({ alg: "RS256", kid: "test-key-1" })
    .setIssuer(opts.iss ?? ISSUER)
    .setAudience(opts.aud ?? CLIENT_ID)
    .setIssuedAt(now)
    .setExpirationTime(now + (opts.expSecondsFromNow ?? 3600))
    .setSubject((claims.sub as string) ?? "operator-123")
    .sign(privateKey);
}

const cfg = {
  allowedIssuers: [ISSUER, "https://oauth.id.jumpcloud.com/"],
  clientId: CLIENT_ID,
  allowedDomains: ALLOWED_DOMAINS,
  jwksResolver: localJwks,
};

Deno.test("valid employee token → verified operator (sub + email)", async () => {
  const token = await makeToken({ sub: "op-abc", email: "mark@adyton.io", email_verified: true });
  const op = await verifyIdToken(token, cfg);
  assertEquals(op.operatorId, "op-abc");
  assertEquals(op.email, "mark@adyton.io");
});

Deno.test("expired token → 401 unauthorized", async () => {
  const token = await makeToken(
    { sub: "op-abc", email: "mark@adyton.io", email_verified: true },
    { expSecondsFromNow: -60 },
  );
  const err = await assertRejects(() => verifyIdToken(token, cfg), OidcError);
  assertEquals(err.status, 401);
});

Deno.test("wrong audience → 401 unauthorized", async () => {
  const token = await makeToken(
    { sub: "op-abc", email: "mark@adyton.io", email_verified: true },
    { aud: "some-other-app" },
  );
  const err = await assertRejects(() => verifyIdToken(token, cfg), OidcError);
  assertEquals(err.status, 401);
});

Deno.test("untrusted issuer → 401 unauthorized", async () => {
  const token = await makeToken(
    { sub: "op-abc", email: "mark@adyton.io", email_verified: true },
    { iss: "https://evil.example.com" },
  );
  const err = await assertRejects(() => verifyIdToken(token, cfg), OidcError);
  assertEquals(err.status, 401);
});

Deno.test("tampered signature → 401 unauthorized", async () => {
  const token = await makeToken({ sub: "op-abc", email: "mark@adyton.io", email_verified: true });
  const tampered = token.slice(0, -4) + "AAAA";
  const err = await assertRejects(() => verifyIdToken(tampered, cfg), OidcError);
  assertEquals(err.status, 401);
});

Deno.test("non-employee domain → 403 forbidden (authenticated but not authorized)", async () => {
  const token = await makeToken({
    sub: "op-xyz",
    email: "stranger@gmail.com",
    email_verified: true,
  });
  const err = await assertRejects(() => verifyIdToken(token, cfg), OidcError);
  assertEquals(err.status, 403);
});

Deno.test("unverified email → 403 forbidden", async () => {
  const token = await makeToken({
    sub: "op-xyz",
    email: "mark@adyton.io",
    email_verified: false,
  });
  const err = await assertRejects(() => verifyIdToken(token, cfg), OidcError);
  assertEquals(err.status, 403);
});

Deno.test("group-based gate admits a non-domain operator in an allowed group", async () => {
  const token = await makeToken({
    sub: "op-grp",
    email: "contractor@vendor.com",
    email_verified: true,
    groups: ["wagner-operators"],
  });
  const op = await verifyIdToken(token, { ...cfg, allowedGroups: ["wagner-operators"] });
  assertEquals(op.operatorId, "op-grp");
});
