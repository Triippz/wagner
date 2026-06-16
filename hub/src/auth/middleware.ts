// T000c — Bearer-token auth middleware (ADR-0002).
//
// Extracts the bearer ID token, verifies it via `verifyIdToken`, and attaches
// the verified operator to the request context. 401 on a missing/invalid token,
// 403 on an authenticated-but-unauthorized operator. Routes downstream read
// `c.get("operator")`.

import type { Context, MiddlewareHandler } from "hono";
import { type OidcConfig, OidcError, verifyIdToken, type VerifiedOperator } from "./oidc.ts";

export interface AuthVariables {
  operator: VerifiedOperator;
}

export function bearerAuth(cfg: OidcConfig): MiddlewareHandler {
  return async (c: Context, next) => {
    const header = c.req.header("authorization") ?? "";
    const match = header.match(/^Bearer\s+(.+)$/i);
    if (!match) {
      return c.json({ error: "missing bearer token" }, 401);
    }
    try {
      const operator = await verifyIdToken(match[1], cfg);
      c.set("operator", operator);
    } catch (e) {
      if (e instanceof OidcError) {
        return c.json({ error: e.code, message: e.message }, e.status);
      }
      return c.json({ error: "auth_error" }, 401);
    }
    await next();
  };
}
