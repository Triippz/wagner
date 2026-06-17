// Presence fan-out interface + implementations.
// Best-effort: errors never propagate to the HTTP response layer.

export interface PresenceFanout {
  announce(peerId: string, noteUid: string): Promise<void>;
  subscribe(noteUid: string, cb: (peerId: string) => void): () => void;
}

/** No-op — used when iroh daemon is unavailable. */
export class NoopPresenceFanout implements PresenceFanout {
  async announce(_peerId: string, _noteUid: string): Promise<void> {}
  subscribe(_noteUid: string, _cb: (p: string) => void) { return () => {}; }
}

/** In-memory fan-out for hermetic tests. */
export class MemoryPresenceFanout implements PresenceFanout {
  private subs = new Map<string, Set<(p: string) => void>>();

  async announce(peerId: string, noteUid: string) {
    this.subs.get(noteUid)?.forEach((cb) => cb(peerId));
  }

  subscribe(noteUid: string, cb: (p: string) => void): () => void {
    if (!this.subs.has(noteUid)) this.subs.set(noteUid, new Set());
    this.subs.get(noteUid)!.add(cb);
    return () => this.subs.get(noteUid)?.delete(cb);
  }
}
