// T000c — OIDC ID-token verification (ADR-0002).
//
// The hub is NOT a credential authority. It validates an IdP-issued ID token
// and derives `operator_id` from the token subject. Two distinct failure modes:
//   - 401 (unauthorized): the token is not a valid, unexpired, correctly-
//     audienced token signed by a trusted issuer. Authentication failed.
//   - 403 (forbidden): the token is valid, but the operator is not authorized
//     (email unverified, or not in an allowed domain/group). Authenticated,
//     not permitted.
// Issuers are Google + JumpCloud (both OIDC) per ADR-0002.

import { createLocalJWKSet, createRemoteJWKSet, type JSONWebKeySet, jwtVerify } from "jose";

export class OidcError extends Error {
  readonly status: 401 | 403;
  readonly code: string;
  constructor(status: 401 | 403, code: string, message: string) {
    super(message);
    this.name = "OidcError";
    this.status = status;
    this.code = code;
  }
}

export interface VerifiedOperator {
  /** Stable IdP subject — the attribution unit across logins. */
  operatorId: string;
  email: string;
  groups: string[];
}

export interface OidcConfig {
  /** Trusted issuers (Google, JumpCloud). A token from any other `iss` → 401. */
  allowedIssuers: string[];
  /** The hub's OIDC client_id — the required `aud`. */
  clientId: string;
  /** Verified-email domains that grant access (employees-only gate). */
  allowedDomains?: string[];
  /** IdP groups that grant access (alternative to domain). */
  allowedGroups?: string[];
  /**
   * Test/seam hook: resolve the JWKS directly (no network). When omitted, the
   * JWKS is fetched remotely from the per-issuer `jwksUris` map.
   */
  jwksResolver?: () => Promise<JSONWebKeySet>;
  /** issuer → jwks_uri, used only when `jwksResolver` is absent. */
  jwksUris?: Record<string, string>;
}

/** Build the jose key-getter from injected JWKS (tests) or a remote URI (prod). */
async function keyGetter(cfg: OidcConfig, issuer: string) {
  if (cfg.jwksResolver) {
    return createLocalJWKSet(await cfg.jwksResolver());
  }
  const uri = cfg.jwksUris?.[issuer];
  if (!uri) {
    throw new OidcError(401, "no_jwks", `no JWKS source for issuer ${issuer}`);
  }
  return createRemoteJWKSet(new URL(uri));
}

/**
 * Verify an OIDC ID token. Resolves to the verified operator on success;
 * rejects with an {@link OidcError} (401 auth / 403 authz) otherwise.
 */
export async function verifyIdToken(
  token: string,
  cfg: OidcConfig,
): Promise<VerifiedOperator> {
  // --- Authentication: signature, issuer, audience, expiry (jose) ---
  let payload: Record<string, unknown>;
  try {
    const issuerHint = decodeIssuer(token);
    const keys = await keyGetter(cfg, issuerHint);
    const result = await jwtVerify(token, keys, {
      issuer: cfg.allowedIssuers,
      audience: cfg.clientId,
    });
    payload = result.payload as Record<string, unknown>;
  } catch (e) {
    if (e instanceof OidcError) throw e;
    throw new OidcError(401, "invalid_token", `token verification failed: ${(e as Error).message}`);
  }

  const sub = typeof payload.sub === "string" ? payload.sub : "";
  const email = typeof payload.email === "string" ? payload.email : "";
  const emailVerified = payload.email_verified === true;
  const groups = Array.isArray(payload.groups)
    ? payload.groups.filter((g): g is string => typeof g === "string")
    : [];

  if (!sub) throw new OidcError(401, "no_subject", "token has no subject");

  // --- Authorization: employees-only gate (domain or group) ---
  if (!emailVerified) {
    throw new OidcError(403, "email_unverified", "email is not verified by the IdP");
  }
  const domainOk = (cfg.allowedDomains ?? []).some((d) => email.toLowerCase().endsWith(`@${d.toLowerCase()}`));
  const groupOk = (cfg.allowedGroups ?? []).some((g) => groups.includes(g));
  if (!domainOk && !groupOk) {
    throw new OidcError(403, "not_employee", "operator is not in an allowed domain or group");
  }

  return { operatorId: sub, email, groups };
}

/** Read the `iss` claim from an unverified token to pick the JWKS source. */
function decodeIssuer(token: string): string {
  const parts = token.split(".");
  if (parts.length !== 3) return "";
  try {
    const json = JSON.parse(new TextDecoder().decode(base64urlDecode(parts[1])));
    return typeof json.iss === "string" ? json.iss : "";
  } catch {
    return "";
  }
}

function base64urlDecode(s: string): Uint8Array {
  const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
  const b64 = (s + pad).replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
