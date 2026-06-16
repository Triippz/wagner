// T000c — Hub entrypoint (Deno.serve). Config from environment only (D-SEC-2):
// no secrets or issuer config inlined. Run: `deno task dev`.

import { createApp } from "./app.ts";

function envList(name: string): string[] {
  return (Deno.env.get(name) ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

function jwksUriMap(): Record<string, string> {
  // ISSUER_JWKS="https://accounts.google.com=https://www.googleapis.com/oauth2/v3/certs,..."
  const out: Record<string, string> = {};
  for (const pair of envList("OIDC_ISSUER_JWKS")) {
    const i = pair.indexOf("=");
    if (i > 0) out[pair.slice(0, i)] = pair.slice(i + 1);
  }
  return out;
}

const app = createApp({
  oidc: {
    allowedIssuers: envList("OIDC_ALLOWED_ISSUERS"),
    clientId: Deno.env.get("OIDC_CLIENT_ID") ?? "",
    allowedDomains: envList("OIDC_ALLOWED_DOMAINS"),
    allowedGroups: envList("OIDC_ALLOWED_GROUPS"),
    jwksUris: jwksUriMap(),
  },
});

const port = Number(Deno.env.get("PORT") ?? "8787");
Deno.serve({ port }, app.fetch);
