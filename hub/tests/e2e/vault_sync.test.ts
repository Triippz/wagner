// Hub vault sync E2E: covers Plan 008 Steps 1-6.
// Same pattern as hub_server.test.ts: real Deno.serve, port 0, OIDC stub, real fetch().
//
// Steps 1-4 (article9, loro_hub, snapshot_store, metadata_store) are tested with
// hermetic in-process instances. Steps 5-6 are tested via real HTTP against the
// full app with memory-injected deps.

import { assertEquals, assertMatch } from "@std/assert";
import { exportJWK, generateKeyPair, type JSONWebKeySet, SignJWT } from "jose";
import { LoroDoc } from "loro-crdt";
import { encodeBase64, decodeBase64 } from "@std/encoding/base64";
import { createApp } from "../../src/app.ts";
import type { AppDeps } from "../../src/app.ts";
import { article9Gate, CURATED_TIERS } from "../../src/vault/article9.ts";
import { LoroHub } from "../../src/vault/loro_hub.ts";
import { MemorySnapshotStore } from "../../src/vault/doc_store.ts";
import { MemoryMetadataStore } from "../../src/vault/metadata_store.ts";
import { MemoryPresenceFanout, NoopPresenceFanout } from "../../src/vault/presence.ts";

// ── OIDC stub (same key as hub_server.test.ts, kept independent) ─────────────

const ISSUER = "https://accounts.google.com";
const CLIENT_ID = "wagner-hub-e2e";

const { publicKey, privateKey } = await generateKeyPair("RS256");
const jwk = await exportJWK(publicKey);
jwk.kid = "e2e-k1";
jwk.alg = "RS256";
const jwks: JSONWebKeySet = { keys: [jwk] };

async function mintToken(claims: Record<string, unknown>): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return new SignJWT(claims)
    .setProtectedHeader({ alg: "RS256", kid: "e2e-k1" })
    .setIssuer(ISSUER)
    .setAudience(CLIENT_ID)
    .setIssuedAt(now)
    .setExpirationTime(now + 3600)
    .setSubject(claims.sub as string)
    .sign(privateKey);
}

// ── Server lifecycle ──────────────────────────────────────────────────────────

interface ServerHandle {
  base: string;
  shutdown: () => Promise<void>;
}

async function startServer(extra?: Partial<AppDeps>): Promise<ServerHandle> {
  const { fetch: handler } = createApp({
    oidc: {
      allowedIssuers: [ISSUER],
      clientId: CLIENT_ID,
      allowedDomains: ["adyton.io"],
      jwksResolver: () => Promise.resolve(jwks),
    },
    vault: {
      snapshotStore: new MemorySnapshotStore(),
      metadataStore: new MemoryMetadataStore(),
      presenceFanout: new MemoryPresenceFanout(),
    },
    ...extra,
  });

  let resolve: (v: { hostname: string; port: number }) => void;
  const ready = new Promise<{ hostname: string; port: number }>((r) => { resolve = r; });

  const server = Deno.serve(
    { port: 0, hostname: "127.0.0.1", onListen: resolve! },
    handler,
  );

  const { hostname, port } = await ready;
  return {
    base: `http://${hostname}:${port}`,
    shutdown: () => server.shutdown(),
  };
}

// helper: produce a minimal loro snapshot as base64
function makeSnapshotB64(content: string): string {
  const doc = new LoroDoc();
  const text = doc.getText("body");
  text.insert(0, content);
  return encodeBase64(doc.export({ mode: "snapshot" }));
}

// ── Step 1: Article IX gate ───────────────────────────────────────────────────

Deno.test("article9: CURATED_TIERS contains insight/decision/reference", () => {
  assertEquals(CURATED_TIERS.has("insight"), true);
  assertEquals(CURATED_TIERS.has("decision"), true);
  assertEquals(CURATED_TIERS.has("reference"), true);
  assertEquals(CURATED_TIERS.has("transcript"), false);
  assertEquals(CURATED_TIERS.has("diff"), false);
});

// ── Step 2: LoroHub ──────────────────────────────────────────────────────────

Deno.test("loro_hub: applyUpdate creates a doc and getSnapshot returns bytes", async () => {
  const hub = new LoroHub();
  const doc = new LoroDoc();
  doc.getText("body").insert(0, "hello");
  const bytes = doc.export({ mode: "snapshot" });
  const snapshot = await hub.applyUpdate("uid-1", bytes);
  assertEquals(snapshot instanceof Uint8Array, true);
  assertEquals(snapshot.length > 0, true);
  assertEquals(hub.getSnapshot("uid-1") !== null, true);
});

Deno.test("loro_hub: concurrent merges converge", async () => {
  const hub = new LoroHub();

  // Two docs with different content — both merge into hub.
  const docA = new LoroDoc();
  docA.getText("body").insert(0, "from-A");
  const docB = new LoroDoc();
  docB.getText("body").insert(0, "from-B");

  await Promise.all([
    hub.applyUpdate("uid-2", docA.export({ mode: "snapshot" })),
    hub.applyUpdate("uid-2", docB.export({ mode: "snapshot" })),
  ]);

  const snap = hub.getSnapshot("uid-2");
  assertEquals(snap !== null, true);

  // Decode the merged snapshot — must be valid loro bytes.
  const merged = new LoroDoc();
  merged.import(snap!);
  const text = merged.getText("body").toString();
  // After convergence the text must include at least one of the contributions.
  assertEquals(text.length > 0, true);
});

Deno.test("loro_hub: 10 concurrent applyUpdate calls do not throw", async () => {
  const hub = new LoroHub();
  const updates = Array.from({ length: 10 }, (_, i) => {
    const d = new LoroDoc();
    d.getText("body").insert(0, `edit-${i}`);
    return hub.applyUpdate("uid-3", d.export({ mode: "snapshot" }));
  });
  await Promise.all(updates);
  assertEquals(hub.size >= 1, true);
});

Deno.test("loro_hub: getSnapshot returns null for unseen uid", () => {
  const hub = new LoroHub();
  assertEquals(hub.getSnapshot("never-seen"), null);
});

// ── Step 3: SnapshotStore ────────────────────────────────────────────────────

Deno.test("snapshot_store: put then get returns same string", async () => {
  const store = new MemorySnapshotStore();
  await store.put("uid-4", "abc123");
  assertEquals(await store.get("uid-4"), "abc123");
});

Deno.test("snapshot_store: missing key returns null", async () => {
  const store = new MemorySnapshotStore();
  assertEquals(await store.get("unknown"), null);
});

Deno.test("snapshot_store: second put overwrites", async () => {
  const store = new MemorySnapshotStore();
  await store.put("uid-5", "first");
  await store.put("uid-5", "second");
  assertEquals(await store.get("uid-5"), "second");
});

// ── Step 4: MetadataStore ─────────────────────────────────────────────────────

Deno.test("metadata_store: upsert and get roundtrip", async () => {
  const store = new MemoryMetadataStore();
  await store.upsert({
    note_uid: "uid-6",
    title: "My Note",
    tier: "insight",
    owner_id: "op-1",
    updated_at: 1000,
    snapshot_b64: "snap",
  });
  const row = await store.get("uid-6");
  assertEquals(row?.title, "My Note");
  assertEquals(row?.tier, "insight");
});

Deno.test("metadata_store: upsert replaces existing row", async () => {
  const store = new MemoryMetadataStore();
  await store.upsert({ note_uid: "uid-7", title: "v1", tier: "insight", owner_id: "op-1", updated_at: 1, snapshot_b64: "" });
  await store.upsert({ note_uid: "uid-7", title: "v2", tier: "decision", owner_id: "op-1", updated_at: 2, snapshot_b64: "" });
  const row = await store.get("uid-7");
  assertEquals(row?.title, "v2");
  assertEquals(row?.tier, "decision");
});

Deno.test("metadata_store: missing key returns null", async () => {
  const store = new MemoryMetadataStore();
  assertEquals(await store.get("never-seen"), null);
});

// ── Step 5: /v1/vault/* routes ────────────────────────────────────────────────

Deno.test("E2E: POST /v1/vault/notes/:uid/update without auth → 401", async () => {
  const srv = await startServer();
  try {
    const res = await fetch(`${srv.base}/v1/vault/notes/uid-x/update`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ tier: "insight", update_b64: makeSnapshotB64("hi") }),
    });
    assertEquals(res.status, 401);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: POST /v1/vault/notes/:uid/update with transcript tier → 403 (Article IX)", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "op-1", email: "a@adyton.io", email_verified: true });
    const res = await fetch(`${srv.base}/v1/vault/notes/uid-x/update`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${tok}` },
      body: JSON.stringify({ tier: "transcript", update_b64: makeSnapshotB64("hi") }),
    });
    assertEquals(res.status, 403);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: POST /v1/vault/notes/:uid/update with insight tier → merged", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "op-1", email: "a@adyton.io", email_verified: true });
    const uid = crypto.randomUUID();
    const res = await fetch(`${srv.base}/v1/vault/notes/${uid}/update`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${tok}` },
      body: JSON.stringify({ tier: "insight", title: "Test Note", update_b64: makeSnapshotB64("content") }),
    });
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body.status, "merged");
    assertEquals(body.note_uid, uid);
    assertEquals(typeof body.snapshot_b64, "string");
    assertEquals(body.snapshot_b64.length > 0, true);
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: GET /v1/vault/notes/:uid/snapshot after update returns bytes", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "op-1", email: "a@adyton.io", email_verified: true });
    const uid = crypto.randomUUID();
    const headers = { "content-type": "application/json", authorization: `Bearer ${tok}` };

    // update first
    await fetch(`${srv.base}/v1/vault/notes/${uid}/update`, {
      method: "POST",
      headers,
      body: JSON.stringify({ tier: "reference", title: "Ref", update_b64: makeSnapshotB64("ref content") }),
    }).then(r => r.body?.cancel());

    // then fetch snapshot
    const res = await fetch(`${srv.base}/v1/vault/notes/${uid}/snapshot`, { headers });
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body.note_uid, uid);
    assertEquals(typeof body.snapshot_b64, "string");
    assertEquals(body.snapshot_b64.length > 0, true);

    // must decode to valid loro bytes
    const snap = decodeBase64(body.snapshot_b64);
    const doc = new LoroDoc();
    doc.import(snap); // must not throw
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: GET /v1/vault/notes/:uid/snapshot for unknown uid → 404", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "op-1", email: "a@adyton.io", email_verified: true });
    const res = await fetch(`${srv.base}/v1/vault/notes/${crypto.randomUUID()}/snapshot`, {
      headers: { authorization: `Bearer ${tok}` },
    });
    assertEquals(res.status, 404);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

// ── Step 6: Presence ──────────────────────────────────────────────────────────

Deno.test("noop_presence: announce resolves without throwing", async () => {
  const p = new NoopPresenceFanout();
  await p.announce("peer-1", "uid-8"); // must not throw
});

Deno.test("memory_presence: subscribe then announce delivers to callback", async () => {
  const p = new MemoryPresenceFanout();
  let received: string | null = null;
  p.subscribe("uid-9", (peerId) => { received = peerId; });
  await p.announce("peer-2", "uid-9");
  assertEquals(received, "peer-2");
});

Deno.test("memory_presence: unsubscribe stops delivery", async () => {
  const p = new MemoryPresenceFanout();
  let count = 0;
  const unsub = p.subscribe("uid-10", () => { count++; });
  await p.announce("peer-3", "uid-10");
  unsub();
  await p.announce("peer-3", "uid-10");
  assertEquals(count, 1);
});

Deno.test("E2E: POST /v1/vault/notes/:uid/presence without auth → 401", async () => {
  const srv = await startServer();
  try {
    const res = await fetch(`${srv.base}/v1/vault/notes/uid-x/presence`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ peer_id: "peer-1" }),
    });
    assertEquals(res.status, 401);
    await res.body?.cancel();
  } finally {
    await srv.shutdown();
  }
});

Deno.test("E2E: POST /v1/vault/notes/:uid/presence with valid auth → ok", async () => {
  const srv = await startServer();
  try {
    const tok = await mintToken({ sub: "op-1", email: "a@adyton.io", email_verified: true });
    const uid = crypto.randomUUID();
    const res = await fetch(`${srv.base}/v1/vault/notes/${uid}/presence`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${tok}` },
      body: JSON.stringify({ peer_id: "peer-1" }),
    });
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body.status, "ok");
  } finally {
    await srv.shutdown();
  }
});
