// Article IX gate middleware — rejects non-curated vault tiers at the hub boundary.
// Curated tiers are the only notes that may leave the edge for the hub.
import type { MiddlewareHandler } from "hono";

export const CURATED_TIERS = new Set(["insight", "decision", "reference"]);

export const article9Gate: MiddlewareHandler = async (c, next) => {
  // Peek at the body tier field without consuming the stream permanently.
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    body = null;
  }
  const tier = (body as Record<string, unknown> | null)?.tier;
  if (tier !== undefined && !CURATED_TIERS.has(tier as string)) {
    return c.json({ error: "Article IX: non-curated tier rejected", tier }, 403);
  }
  // Stash parsed body so route handlers can read it without re-parsing.
  c.set("parsedBody", body);
  await next();
};
