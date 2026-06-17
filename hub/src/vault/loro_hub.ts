// Authoritative in-memory LoroDoc map for the hub.
// One LoroDoc per note UUID; merges are queued per-note to prevent concurrent races.
import { LoroDoc } from "loro-crdt";

export class LoroHub {
  // ponytail: plain Maps — no class hierarchy needed for one impl
  private docs = new Map<string, LoroDoc>();
  private queues = new Map<string, Promise<void>>();

  /** Apply an incoming loro update (bytes) to the authoritative doc for noteUid.
   *  Returns the merged snapshot bytes. Queued per-note for sequential application. */
  async applyUpdate(noteUid: string, updateBytes: Uint8Array): Promise<Uint8Array> {
    const prev = this.queues.get(noteUid) ?? Promise.resolve();
    let snapshot!: Uint8Array;
    const next = prev.then(async () => {
      const doc = this.docs.get(noteUid) ?? new LoroDoc();
      doc.import(updateBytes);
      this.docs.set(noteUid, doc);
      snapshot = doc.export({ mode: "snapshot" });
    });
    this.queues.set(noteUid, next);
    await next;
    return snapshot;
  }

  /** Current snapshot for noteUid, or null if unseen. */
  getSnapshot(noteUid: string): Uint8Array | null {
    const doc = this.docs.get(noteUid);
    return doc ? doc.export({ mode: "snapshot" }) : null;
  }

  get size(): number { return this.docs.size; }
}
