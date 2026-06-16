// Discovery / signaling routes (T013, FR-201/212, R-3).
//
// register = the hub side of edge ARMING: a verified operator advertises their
// host's iroh NodeId + signaling ticket. resolve = owner-only lookup a verified
// peer uses to attach. The hub does signaling ONLY — no run data crosses here
// (Article VI; F-1). Arming is edge-driven; there is no route by which a peer
// arms someone else's host (self-arm is structurally impossible — FR-201).

import { Hono } from "hono";
import type { AuthVariables } from "../auth/middleware.ts";
import type { DiscoveryRegistry } from "../discovery/registry.ts";

/** Default arming TTL when the client omits one (ms). */
const DEFAULT_TTL_MS = 60_000;
/** Clamp TTL so a stale registration can't linger indefinitely. */
const MAX_TTL_MS = 5 * 60_000;

interface RegisterBody {
  node_id?: unknown;
  ticket?: unknown;
  ttl_ms?: unknown;
}

interface ResolveBody {
  operator_id?: unknown;
}

export function discoveryRoutes(registry: DiscoveryRegistry): Hono<{ Variables: AuthVariables }> {
  const r = new Hono<{ Variables: AuthVariables }>();

  // Arm: register the caller's own host. operator_id is the VERIFIED subject,
  // never taken from the body — a peer cannot arm on another's behalf.
  r.post("/register", async (c) => {
    const operator = c.get("operator");
    const body = (await c.req.json().catch(() => ({}))) as RegisterBody;
    if (typeof body.node_id !== "string" || typeof body.ticket !== "string") {
      return c.json({ error: "invalid_body", message: "node_id and ticket are required strings" }, 400);
    }
    const ttlMs = typeof body.ttl_ms === "number" && body.ttl_ms > 0
      ? Math.min(body.ttl_ms, MAX_TTL_MS)
      : DEFAULT_TTL_MS;
    registry.register({
      operatorId: operator.operatorId,
      nodeId: body.node_id,
      ticket: body.ticket,
      ttlMs,
    });
    return c.json({ status: "armed", operator_id: operator.operatorId });
  });

  // Resolve: owner-only. The verified requester may resolve only their OWN host.
  // 404 (not 403) for a non-owner or never-armed operator — don't disclose
  // whether someone else's host is armed.
  r.post("/resolve", async (c) => {
    const operator = c.get("operator");
    const body = (await c.req.json().catch(() => ({}))) as ResolveBody;
    if (typeof body.operator_id !== "string") {
      return c.json({ error: "invalid_body", message: "operator_id is required" }, 400);
    }
    const reg = registry.resolve({ operatorId: body.operator_id, requesterId: operator.operatorId });
    if (!reg) return c.json({ error: "not_found" }, 404);
    return c.json({ node_id: reg.nodeId, ticket: reg.ticket, expires_at: reg.expiresAt });
  });

  return r;
}
