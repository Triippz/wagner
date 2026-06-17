// Note metadata store interface + in-memory implementation.
// All fields are plain scalars (SurrealDB 2.x constraint: no nested objects/enums).

export interface NoteMetaRow {
  note_uid: string;     // primary key — plain string
  title: string;
  tier: string;         // plain string, not an enum type
  owner_id: string;
  updated_at: number;   // Unix epoch ms
  snapshot_b64: string; // base64-encoded loro snapshot
}

export interface MetadataStore {
  upsert(row: NoteMetaRow): Promise<void>;
  get(noteUid: string): Promise<NoteMetaRow | null>;
}

/** In-memory stub for hermetic tests. */
export class MemoryMetadataStore implements MetadataStore {
  private m = new Map<string, NoteMetaRow>();
  async upsert(r: NoteMetaRow) { this.m.set(r.note_uid, r); }
  async get(k: string) { return this.m.get(k) ?? null; }
}
