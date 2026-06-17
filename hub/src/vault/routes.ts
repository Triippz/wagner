// /v1/vault/* Hono sub-app. Mounted by app.ts at /v1/vault.
// Depends on: article9Gate, LoroHub, SnapshotStore, MetadataStore, PresenceFanout.
import { Hono } from "hono";
import { encodeBase64, decodeBase64 } from "@std/encoding/base64";
import type { AuthVariables } from "../auth/middleware.ts";
import { article9Gate } from "./article9.ts";
import { LoroHub } from "./loro_hub.ts";
import type { SnapshotStore } from "./doc_store.ts";
import type { MetadataStore } from "./metadata_store.ts";
import type { PresenceFanout } from "./presence.ts";

export interface VaultDeps {
  snapshotStore: SnapshotStore;
  metadataStore: MetadataStore;
  presenceFanout: PresenceFanout;
}

// ponytail: single LoroHub instance shared across requests (shared mutable state,
// acceptable because Deno is single-threaded and hub is a singleton process)
const loroHub = new LoroHub();

export function vaultRoutes(deps: VaultDeps) {
  const app = new Hono<{ Variables: AuthVariables & { parsedBody: unknown } }>();

  // POST /v1/vault/notes/:uid/update — merge a loro delta into the authoritative doc
  app.post("/notes/:uid/update", article9Gate, async (c) => {
    const uid = c.req.param("uid");
    const operator = c.get("operator");
    // parsedBody was parsed by article9Gate
    const body = c.get("parsedBody") as {
      tier: string;
      update_b64: string;
      title?: string;
    } | null;

    if (!body?.update_b64) {
      return c.json({ error: "update_b64 is required" }, 400);
    }

    let updateBytes: Uint8Array;
    try {
      updateBytes = decodeBase64(body.update_b64);
    } catch {
      return c.json({ error: "update_b64 must be valid base64" }, 400);
    }

    const mergedBytes = await loroHub.applyUpdate(uid, updateBytes);
    const snapshot_b64 = encodeBase64(mergedBytes);

    await deps.snapshotStore.put(uid, snapshot_b64);
    await deps.metadataStore.upsert({
      note_uid: uid,
      title: body.title ?? uid,
      tier: body.tier,
      owner_id: operator.operatorId,
      updated_at: Date.now(),
      snapshot_b64,
    });

    return c.json({ status: "merged", note_uid: uid, snapshot_b64 });
  });

  // GET /v1/vault/notes/:uid/snapshot — fetch the latest merged snapshot
  app.get("/notes/:uid/snapshot", async (c) => {
    const uid = c.req.param("uid");
    const snapshot_b64 = await deps.snapshotStore.get(uid);
    if (snapshot_b64 === null) {
      return c.json({ error: "note not found", note_uid: uid }, 404);
    }
    return c.json({ note_uid: uid, snapshot_b64 });
  });

  // POST /v1/vault/notes/:uid/presence — best-effort peer presence announcement
  app.post("/notes/:uid/presence", async (c) => {
    const uid = c.req.param("uid");
    let peer_id: string | undefined;
    try {
      const body = await c.req.json() as { peer_id?: string };
      peer_id = body.peer_id;
    } catch { /* swallow — presence is best-effort */ }

    if (peer_id) {
      // Fire-and-forget; errors must not surface to the HTTP layer.
      deps.presenceFanout.announce(peer_id, uid).catch(() => {});
    }
    return c.json({ status: "ok" });
  });

  return app;
}
