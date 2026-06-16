// T000c — Hub application factory (Deno + Hono; ADR-0001).
//
// Minimum-viable wedge-001 hub surface that wedge-002 builds discovery on top
// of: a health probe + the OIDC bearer-auth seam + the ephemeral discovery
// registry, all injected so tests run without a live IdP or SurrealDB. The
// durable SurrealDB tables (operators, project_enrollment) and the 001
// sync/recall routes are deferred to the full 001 hub build.

import { Hono } from "hono";
import type { OidcConfig } from "./auth/oidc.ts";
import { type AuthVariables, bearerAuth } from "./auth/middleware.ts";
import { DiscoveryRegistry } from "./discovery/registry.ts";
import { discoveryRoutes } from "./routes/discovery.ts";

export interface AppDeps {
  oidc: OidcConfig;
  registry?: DiscoveryRegistry;
}

export interface App {
  fetch: (req: Request) => Response | Promise<Response>;
  registry: DiscoveryRegistry;
}

export function createApp(deps: AppDeps): App {
  const registry = deps.registry ?? new DiscoveryRegistry();
  const app = new Hono<{ Variables: AuthVariables }>();

  // Unauthenticated liveness probe.
  app.get("/health", (c) => c.json({ status: "ok", service: "wagner-hub" }));

  // Everything below /v1 requires a verified operator. wedge-002 mounts the
  // discovery routes here (T013); this establishes the auth seam they inherit.
  const v1 = new Hono<{ Variables: AuthVariables }>();
  v1.use("*", bearerAuth(deps.oidc));
  v1.get("/whoami", (c) => c.json({ operator: c.get("operator") }));
  v1.route("/discovery", discoveryRoutes(registry));
  app.route("/v1", v1);

  return { fetch: app.fetch, registry };
}
