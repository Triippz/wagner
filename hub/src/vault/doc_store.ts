// Snapshot store interface + in-memory implementation.
// The live IrohDocsStore (wrapping the iroh daemon HTTP API) is defined but not
// tested here — hermetic tests inject MemorySnapshotStore.

export interface SnapshotStore {
  put(noteUid: string, snapshotB64: string): Promise<void>;
  get(noteUid: string): Promise<string | null>;
}

/** Hermetic in-memory stub used in tests and as the default when no iroh daemon is configured. */
export class MemorySnapshotStore implements SnapshotStore {
  private m = new Map<string, string>();
  async put(k: string, v: string) { this.m.set(k, v); }
  async get(k: string) { return this.m.get(k) ?? null; }
}

/** Live client wrapping the iroh daemon's HTTP API (not used in hermetic tests). */
export class IrohDocsStore implements SnapshotStore {
  constructor(private baseUrl: string, private namespace: string) {}

  async put(noteUid: string, snapshotB64: string): Promise<void> {
    await fetch(`${this.baseUrl}/v0/docs/${this.namespace}/entries`, {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ key: noteUid, value: snapshotB64 }),
    });
  }

  async get(noteUid: string): Promise<string | null> {
    const res = await fetch(`${this.baseUrl}/v0/docs/${this.namespace}/entries/${noteUid}`);
    if (res.status === 404) { await res.body?.cancel(); return null; }
    const body = await res.json() as { value?: string };
    return body.value ?? null;
  }
}
