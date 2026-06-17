# Plan 006 — Distributed Sync v1: loro + iroh + the file↔CRDT projector

> Phase 5 of the roadmap. Source: `docs/wagner-vision-and-architecture.md` §5/§9,
> `handoff.md` §5/§9, `build-context.md` §Roadmap, ADR-0003.
> Baseline: `make verify` → exit 0. Re-run before starting.
> Prerequisite: Plan 004 (Vault v1) fully committed and green.
> This plan is **not** parallel-capable with other code-touching work; the projector
> is load-bearing and the cargo dependency graph changes are broad.

---

## Overview

Plan 004 gave the vault a local kernel: `MemoryStore` over embedded SurrealDB with
`save_note` / wikilink / backlink / staging. **Plan 006 makes that kernel multi-writer
and distributed**, adding:

1. **`loro`** — one `LoroDoc` per note (bounded merge scope, per-note time-travel);
   one shared `LoroTree` for the folder hierarchy (UUID node ids, concurrent
   rename/move handled by the movable-tree CRDT); one shared `LoroMap` for the
   UUID↔display-name index (wikilink integrity under concurrent rename).
2. **`iroh-gossip`** — realtime delta delivery per open note (50–500 B/keystroke-batch).
3. **`iroh-docs`** — durable snapshot store (catch-up for offline peers), one
   namespace per vault, keyed by note UUID. **This is only the snapshot store; the
   authoritative merge lives in the in-memory `LoroDoc` at the hub (Plan 008).**
4. **`iroh-blobs`** — attachment sync (binary files linked from notes).
5. **The file↔CRDT projector** — `notify-rs` watcher, bidirectional bridge between
   on-disk `.md` files and `LoroDoc` ops. **Highest-risk component; flagged below.**
6. **`PresenceBroadcaster`** trait — thin awareness layer over iroh-gossip, behind a
   trait so failures degrade to "no presence."

The sync layer rides on the **existing `edge/host/src/remote/`** iroh seam
(`endpoint.rs`, `arm.rs`, `session.rs`, `control.rs`) — it reuses the `iroh::Endpoint`
and the ALPN channel convention rather than standing up a second transport.

**SurrealDB constraint:** all persisted fields remain **plain scalars** (strings,
numbers, arrays of scalars). The loro snapshot bytes are stored as a base64-encoded
string, not a nested object — SurrealDB 2.x's content serializer rejects enums,
`Option`, and nested objects.

---

## Architecture diagram

```mermaid
flowchart TD
    subgraph EDGE ["Edge (Tauri app)"]
        MD["`.wagner/memory/<uid>.md`\non-disk files"]
        PROJ["Projector\n(notify-rs watcher)"]
        VAULT_CRDT["VaultCrdt\n• LoroDoc per note\n• LoroTree (folders)\n• LoroMap (uuid→name)"]
        SYNC["SyncAdapter\n• iroh-gossip (realtime)\n• iroh-docs (catch-up)\n• iroh-blobs (attachments)"]
        MD <-->|"file-lock + CAS\non last-projected hash"| PROJ
        PROJ <-->|"diff into ops /\nmerge export"| VAULT_CRDT
        VAULT_CRDT <-->|"delta bytes"| SYNC
    end

    subgraph HUB ["Hub (always-on peer — Plan 008)"]
        IROH_DOCS["iroh-docs store\n(snapshot per note UUID)"]
        LORO_MEM["LoroDoc in-memory\n(authoritative merge)"]
        IROH_DOCS <-->|"snapshot I/O"| LORO_MEM
    end

    SYNC <-->|"QUIC / iroh gossip\n+ iroh-docs namespace"| IROH_DOCS
    SYNC <-.->|"gossip deltas"| PEER["Teammate (same stack)"]
```

---

## Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| **Projector race** — Obsidian writes a file during an incoming loro merge write | **HIGH** | File lock (platform advisory lock on the `.md`); CAS on `last_projected_hash` stored in `VaultCrdt`; debounce on both directions; wide deterministic test harness (see Step 7). |
| iroh-docs LWW vs loro ordering at the hub | MEDIUM | Hub's in-memory `LoroDoc` is the authoritative merge; iroh-docs is snapshot-only (Plan 008). |
| loro awareness gap in Rust | LOW | `PresenceBroadcaster` trait wraps iroh-gossip; failures degrade gracefully. |
| iroh-gossip topic size under many open notes | LOW | One gossip topic per note UUID; inactive notes produce no traffic. |

---

## New Cargo dependencies

Add to `Cargo.toml` `[workspace.dependencies]` and to `edge/host/Cargo.toml`
`[dependencies]`:

```toml
loro         = "1"          # CRDT engine (LoroDoc, LoroText, LoroTree, LoroMap)
iroh-gossip  = "0.27"       # realtime delta pub/sub over iroh QUIC
iroh-docs    = "0.27"       # durable snapshot store (keyed by note UUID)
iroh-blobs   = "0.27"       # blob/attachment sync
notify       = "6"          # cross-platform file-system watcher (notify-rs)
base64       = "0.22"       # CRDT snapshot → plain scalar string for SurrealDB
```

> `iroh = "1"` is already in the workspace; gossip/docs/blobs are companion crates
> from the same iroh 0.27 release family. Check exact compatible versions with
> `cargo add iroh-gossip --dry-run` and pin them.

---

## Step 1 — Dependency integration smoke test

**What:** Add `loro`, `iroh-gossip`, `iroh-docs`, `iroh-blobs`, `notify`, `base64` to
`Cargo.toml` (workspace) and `edge/host/Cargo.toml`. Write a minimal compile-time
smoke test that constructs a `LoroDoc` and imports the key types from each crate,
asserting they resolve.

**Files:**
- `Cargo.toml` (workspace `[workspace.dependencies]`)
- `edge/host/Cargo.toml`
- `edge/host/tests/unit/crdt_smoke.rs` (new; `#[test] fn loro_doc_constructs()`)

**Tests (RED first):**
```rust
// edge/host/tests/unit/crdt_smoke.rs
#[test]
fn loro_doc_constructs_and_text_ops() {
    use loro::{LoroDoc, LoroText};
    let doc = LoroDoc::new();
    let text = doc.get_text("body");
    text.insert(0, "hello").unwrap();
    let snapshot = doc.export(loro::ExportMode::Snapshot);
    assert!(!snapshot.is_empty());
}
```

**Acceptance:** `make cargo` (including `make clippy`) exits 0. No unused-import
warnings. The existing 231 tests remain green.

---

## Step 2 — `VaultCrdt` module skeleton

**What:** New module `edge/host/src/vault/crdt.rs` (add `pub mod crdt;` to
`edge/host/src/vault/mod.rs`). Define `VaultCrdt` struct holding:
- `HashMap<String, LoroDoc>` — per-note docs keyed by note UUID.
- `LoroDoc` — the shared folder-hierarchy doc (one `LoroTree` container named
  `"folders"`).
- `LoroDoc` — the shared name-index doc (one `LoroMap` container named
  `"name_index"`; maps UUID string → display-name string).

Public API (stubs only at this step — `todo!()` bodies; tests drive the real impl
in later steps):
```rust
pub struct VaultCrdt { /* private fields */ }

impl VaultCrdt {
    pub fn new() -> Self;

    /// Open (or create) a LoroDoc for `note_uid`. Returns a reference.
    pub fn note_doc(&mut self, note_uid: &str) -> &mut LoroDoc;

    /// Snapshot bytes for `note_uid` suitable for iroh-docs storage.
    /// Returns base64-encoded loro snapshot (plain scalar — SurrealDB constraint).
    pub fn note_snapshot(&self, note_uid: &str) -> Option<String>;

    /// Apply an incoming merge (gossip delta or catch-up snapshot) for `note_uid`.
    pub fn apply_note_update(&mut self, note_uid: &str, bytes: &[u8])
        -> Result<(), CrdtError>;

    /// Export the current text of a note as a plain String (for projector → disk).
    pub fn note_text(&self, note_uid: &str) -> Option<String>;

    /// The folder tree as a flat `Vec<(uuid, parent_uuid, display_name)>`.
    pub fn folder_entries(&self) -> Vec<(String, Option<String>, String)>;

    /// Upsert a UUID→display-name mapping in the shared name-index doc.
    pub fn index_name(&mut self, uuid: &str, display_name: &str);

    /// Resolve a display name to a UUID (or None).
    pub fn resolve_name(&self, display_name: &str) -> Option<String>;
}

#[derive(Debug, thiserror::Error)]
pub enum CrdtError {
    #[error("invalid loro snapshot bytes")]
    InvalidSnapshot(#[from] loro::LoroError),
    #[error("base64 decode error")]
    Base64(#[from] base64::DecodeError),
}
```

**Tests (RED first, inline in `crdt.rs`):**
- `vault_crdt_creates_note_doc` — `note_doc("uid-a")` returns a doc; `note_text`
  returns `None` for a fresh doc.
- `vault_crdt_snapshot_roundtrip` — write text, snapshot, reconstruct in a second
  `VaultCrdt`, verify text matches.
- `vault_crdt_name_index_roundtrip` — `index_name` + `resolve_name` round-trips.

**Acceptance:** compiles + tests green; `make cargo` exits 0.

---

## Step 3 — CRDT merge logic (in-memory, no network)

**What:** Implement `apply_note_update`, `note_text`, and the merge semantics using
`LoroDoc::import`. Demonstrate correct concurrent-edit merge with no real network.

**Files:** `edge/host/src/vault/crdt.rs` (implement previously-stubbed methods),
`edge/host/tests/unit/crdt_merge.rs` (new).

**Tests (RED first — deterministic, in-memory loro only):**
```rust
// edge/host/tests/unit/crdt_merge.rs

/// Two simulated peers edit the same note concurrently; loro merges both edits.
#[test]
fn concurrent_edits_merge_without_data_loss() {
    use loro::{LoroDoc, ExportMode};
    let mut peer_a = LoroDoc::new();
    let mut peer_b = LoroDoc::new();

    // Shared history: both peers start from the same snapshot.
    let text_a = peer_a.get_text("body");
    text_a.insert(0, "Hello").unwrap();
    let shared = peer_a.export(ExportMode::Snapshot);
    peer_b.import(&shared).unwrap();

    // Concurrent edit A: append " world"
    peer_a.get_text("body").insert(5, " world").unwrap();
    // Concurrent edit B: prepend "Say: "
    peer_b.get_text("body").insert(0, "Say: ").unwrap();

    // Merge A into B and B into A — both must converge.
    let update_a = peer_a.export(ExportMode::Updates { from: Default::default() });
    let update_b = peer_b.export(ExportMode::Updates { from: Default::default() });
    peer_a.import(&update_b).unwrap();
    peer_b.import(&update_a).unwrap();

    let text_in_a = peer_a.get_text("body").to_string();
    let text_in_b = peer_b.get_text("body").to_string();
    assert_eq!(text_in_a, text_in_b, "peers converge");
    assert!(text_in_a.contains("Hello"), "original text preserved");
    assert!(text_in_a.contains("world"), "A's edit preserved");
    assert!(text_in_a.contains("Say:"), "B's edit preserved");
}

/// Applying the same update twice is idempotent.
#[test]
fn apply_note_update_is_idempotent() { /* ... */ }

/// VaultCrdt.apply_note_update rejects garbage bytes with CrdtError.
#[test]
fn apply_note_update_rejects_invalid_bytes() { /* ... */ }
```

**Acceptance:** all merge tests pass, no panics, `make cargo` green.

---

## Step 4 — `LoroTree` folder hierarchy + concurrent rename

**What:** Implement `folder_entries()` and add `move_note(uid, new_parent)` /
`rename_note(uid, new_name)` to `VaultCrdt`. Use `LoroDoc::get_tree("folders")` —
the movable-tree CRDT handles concurrent renames/moves without conflict.

**Files:** `edge/host/src/vault/crdt.rs` (extend), `edge/host/tests/unit/crdt_tree.rs`
(new).

**Tests (RED first):**
- `tree_concurrent_move_converges` — two peers each move the same note to different
  parents; after merge both trees are identical (movable-tree resolves to one winner).
- `tree_rename_propagates` — rename a node; export/import to a second doc; new name
  present.
- `folder_entries_reflects_tree_state` — `folder_entries()` returns a flat
  `Vec<(uuid, Option<parent_uuid>, name)>` matching the tree.

**Acceptance:** tests green; `make cargo` exits 0.

---

## Step 5 — `SyncAdapter` trait + iroh-gossip delta transport

**What:** New `edge/host/src/vault/sync.rs` (add `pub mod sync;` in
`edge/host/src/vault/mod.rs`). Define the `SyncAdapter` trait and a
`GossipSyncAdapter` implementation that wraps iroh-gossip for realtime delta
delivery per open note.

```rust
// edge/host/src/vault/sync.rs

#[async_trait::async_trait]
pub trait SyncAdapter: Send + Sync {
    /// Broadcast a loro update delta for `note_uid` to all connected peers.
    async fn broadcast_update(&self, note_uid: &str, delta: Vec<u8>)
        -> Result<(), SyncError>;

    /// Subscribe to incoming updates for `note_uid`.
    /// The returned channel receives raw loro update bytes.
    async fn subscribe(&self, note_uid: &str)
        -> Result<tokio::sync::mpsc::Receiver<Vec<u8>>, SyncError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError { /* ... */ }

/// In-memory adapter for tests — no iroh, no network.
pub struct MemorySyncAdapter {
    // Arc<Mutex<HashMap<note_uid, Vec<Sender>>>> — fan-out to all subscribers
}
```

The `GossipSyncAdapter` (iroh-gossip live implementation) is **integration-only
code** — it is not unit-tested with a live iroh node. The interface is tested via
`MemorySyncAdapter`.

**Files:**
- `edge/host/src/vault/sync.rs` (new)
- `edge/host/tests/unit/sync_adapter.rs` (new)

**Tests (RED first — `MemorySyncAdapter` only):**
- `memory_adapter_broadcasts_and_subscriber_receives` — broadcast + subscribe
  round-trip; subscriber channel yields the sent bytes.
- `memory_adapter_multiple_subscribers_all_receive` — fan-out to two subscribers.
- `memory_adapter_subscribe_after_broadcast_misses_past` — no history; new
  subscriber does not receive old messages (iroh-gossip is not a log).

**Acceptance:** trait object-safe; `MemorySyncAdapter` tests green; `GossipSyncAdapter`
stub compiles (can be `todo!()` body); `make cargo` green.

---

## Step 6 — iroh-docs catch-up store integration

**What:** Add `DocsStore` to `edge/host/src/vault/sync.rs`. Wraps iroh-docs for
durable note snapshots (catch-up after offline). The docs namespace is shared with
the hub (Plan 008 mounts the same namespace). Key = note UUID, value = base64-encoded
loro snapshot (plain scalar constraint).

```rust
/// Abstraction over iroh-docs for snapshot catch-up.
#[async_trait::async_trait]
pub trait SnapshotStore: Send + Sync {
    async fn put_snapshot(&self, note_uid: &str, snapshot_b64: &str)
        -> Result<(), SyncError>;
    async fn get_snapshot(&self, note_uid: &str)
        -> Result<Option<String>, SyncError>;
}

/// In-memory snapshot store for tests.
pub struct MemorySnapshotStore {
    inner: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}
```

The live `IrohDocsStore` wraps iroh-docs and is integration-only (no unit test with
a live iroh node).

**Tests (RED first — `MemorySnapshotStore`):**
- `snapshot_store_put_and_get_roundtrip` — put a base64 snapshot string; get
  returns it.
- `snapshot_store_missing_key_returns_none`
- `snapshot_store_overwrite_replaces` — second put with same key overwrites.

**Acceptance:** trait and in-memory impl tests green; live `IrohDocsStore` stub
compiles; `make cargo` green.

---

## Step 7 — THE PROJECTOR: file↔CRDT bidirectional bridge

> **Highest-risk component.** This step has the largest test surface and must not
> be compressed. Budget at least 2x the test-writing time of any other step.
> Use **opus** (not sonnet) for the hard implementation decisions here.

**What:** `edge/host/src/vault/projector.rs` (add `pub mod projector;` in
`edge/host/src/vault/mod.rs`). A `notify-rs` watcher bridges on-disk `.md` files
and their `LoroDoc`:

```
DISK EDIT (Obsidian writes file)
  → notify event
  → Projector::on_disk_change()
  → acquire file advisory lock (std::fs::File + fcntl/LockFileEx)
  → read file content
  → compute diff vs. last_projected_text
  → emit LoroText ops into VaultCrdt::note_doc()
  → release lock
  → update last_projected_hash

LORO MERGE (incoming update applied to VaultCrdt)
  → Projector::on_crdt_merge()
  → ONLY proceed if no_active_local_edit (debounce + last_edit_timestamp)
  → acquire file advisory lock
  → CAS: read file, hash, compare vs. stored last_projected_hash
  → if hash matches → write new content atomically (temp file + rename)
  → update last_projected_hash
  → release lock
  → IF hash mismatch → skip write, log collision, re-schedule for next idle window
```

```rust
// edge/host/src/vault/projector.rs

pub struct Projector {
    crdt: Arc<tokio::sync::Mutex<VaultCrdt>>,
    /// Per-note: last content hash projected to disk (for CAS guard).
    projected_hashes: HashMap<String, [u8; 32]>,
    /// Per-note: timestamp of last local keystroke event (debounce).
    last_local_edit: HashMap<String, std::time::Instant>,
    /// Debounce window: no outbound merge write within this duration of a local edit.
    debounce: std::time::Duration,
}

impl Projector {
    pub fn new(crdt: Arc<tokio::sync::Mutex<VaultCrdt>>, debounce: std::time::Duration) -> Self;

    /// Called by the notify watcher on a file change event for `note_uid`.
    /// Returns `Ok(delta)` = the loro update bytes to broadcast, or `None`
    /// if the change was already reflected (no-op).
    pub async fn on_disk_change(
        &mut self,
        note_uid: &str,
        file_path: &std::path::Path,
    ) -> Result<Option<Vec<u8>>, ProjectorError>;

    /// Called after applying an incoming merge to VaultCrdt.
    /// Writes the merged text back to disk ONLY when no local edit is in flight.
    pub async fn on_crdt_merge(
        &mut self,
        note_uid: &str,
        file_path: &std::path::Path,
    ) -> Result<(), ProjectorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectorError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CRDT error: {0}")]
    Crdt(#[from] CrdtError),
    #[error("hash mismatch — merge write skipped (collision)")]
    HashMismatch,
}
```

**Tests (RED first — deterministic harness, NO real notify watcher, NO real network):**

All tests in `edge/host/tests/unit/projector.rs` drive `Projector` directly by
calling `on_disk_change` / `on_crdt_merge` with in-memory `LoroDoc` instances and
`tempfile` directories. No network, no gossip, no iroh.

```rust
// edge/host/tests/unit/projector.rs

/// Basic disk→CRDT flow: writing a file and calling on_disk_change should
/// produce a loro delta with the new text.
#[tokio::test]
async fn disk_change_produces_crdt_delta() { /* ... */ }

/// Basic CRDT→disk flow: merge arrives when no local edit in flight;
/// file is overwritten with the merged content.
#[tokio::test]
async fn crdt_merge_writes_to_disk_when_idle() { /* ... */ }

/// CRDT→disk write is SKIPPED when a local edit just occurred (debounce window).
#[tokio::test]
async fn crdt_merge_skipped_during_debounce() { /* ... */ }

/// CAS guard: if someone else modified the file between our last projection
/// and the merge write, on_crdt_merge returns HashMismatch (no silent corruption).
#[tokio::test]
async fn cas_guard_detects_external_modification() { /* ... */ }

/// Concurrent simulation: two "peers" (in-memory LoroDoc instances) apply
/// independent edits in alternation; projector converges both without data loss.
#[tokio::test]
async fn concurrent_peer_edits_converge_via_projector() {
    // Peer A edits → disk_change → delta → apply to B.
    // Peer B edits → disk_change → delta → apply to A.
    // After N rounds: A's disk file == B's CRDT text. No panic.
    /* ... */
}

/// Wikilink rename: after a name-index update, on_crdt_merge rewrites stale
/// [[OldName]] references to [[NewName]] in the projected markdown.
#[tokio::test]
async fn rename_rewrites_stale_wikilinks() { /* ... */ }

/// No-op: projector does not re-emit a delta when the file content
/// matches the already-projected text (avoids echo loops).
#[tokio::test]
async fn disk_change_is_noop_when_content_unchanged() { /* ... */ }
```

**Acceptance:** all 6 projector tests pass; `make cargo` + `make clippy` exit 0;
no `unwrap()` on the projector path (all errors are typed `ProjectorError`); the
concurrent test must be deterministic (no `sleep`, no real file-system race).

---

## Step 8 — `PresenceBroadcaster` trait + gossip-backed impl stub

**What:** `edge/host/src/vault/presence.rs` (add `pub mod presence;` in
`edge/host/src/vault/mod.rs`). The loro Rust crate has no awareness/presence; this
is a thin wrapper over iroh-gossip that broadcasts cursor/typing presence events.

```rust
#[async_trait::async_trait]
pub trait PresenceBroadcaster: Send + Sync {
    /// Broadcast that `peer_id` is currently editing `note_uid`.
    async fn announce_editing(&self, peer_id: &str, note_uid: &str)
        -> Result<(), PresenceError>;

    /// Subscribe to presence events. Channel yields `(peer_id, note_uid)` pairs.
    async fn presence_events(&self)
        -> Result<tokio::sync::mpsc::Receiver<(String, String)>, PresenceError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PresenceError {
    #[error("presence unavailable: {0}")]
    Unavailable(String),
}

/// No-op implementation — presence is optional; failures degrade gracefully.
pub struct NoopPresence;

/// In-memory broadcaster for tests.
pub struct MemoryPresenceBroadcaster { /* ... */ }
```

**Tests (RED first):**
- `memory_presence_announce_and_subscribe` — announce; subscriber receives
  `(peer_id, note_uid)`.
- `noop_presence_announce_returns_ok` — `NoopPresence::announce_editing` never errors.

**Acceptance:** trait tests green; `make cargo` green.

---

## Step 9 — Integration: `VaultSync` orchestrator

**What:** `edge/host/src/vault/vault_sync.rs` (add `pub mod vault_sync;`). This ties
`VaultCrdt` + `SyncAdapter` + `SnapshotStore` + `Projector` + `PresenceBroadcaster`
into a single `VaultSync` struct that an IPC command handler constructs.

```rust
pub struct VaultSync {
    crdt: Arc<tokio::sync::Mutex<VaultCrdt>>,
    projector: Projector,
    sync: Arc<dyn SyncAdapter>,
    snapshots: Arc<dyn SnapshotStore>,
    presence: Arc<dyn PresenceBroadcaster>,
    project_dir: std::path::PathBuf,
}

impl VaultSync {
    pub fn new(
        project_dir: std::path::PathBuf,
        sync: Arc<dyn SyncAdapter>,
        snapshots: Arc<dyn SnapshotStore>,
        presence: Arc<dyn PresenceBroadcaster>,
        debounce: std::time::Duration,
    ) -> Self;

    /// Called by the IPC layer when a note is saved locally.
    pub async fn on_local_save(&mut self, note_uid: &str) -> Result<(), VaultSyncError>;

    /// Called by the gossip receiver task when a delta arrives.
    pub async fn on_remote_delta(&mut self, note_uid: &str, delta: &[u8])
        -> Result<(), VaultSyncError>;

    /// Catch-up for a note after coming back online (reads iroh-docs snapshot).
    pub async fn catch_up_note(&mut self, note_uid: &str) -> Result<(), VaultSyncError>;
}
```

**Tests (RED first, `edge/host/tests/integration/vault_sync.rs`):**
- `local_save_broadcasts_delta` — `on_local_save` calls
  `SyncAdapter::broadcast_update` (verified via `MemorySyncAdapter`).
- `remote_delta_updates_disk` — `on_remote_delta` applies the delta and calls
  `Projector::on_crdt_merge` which writes to disk.
- `catch_up_from_snapshot` — `catch_up_note` reads from `MemorySnapshotStore` and
  applies the snapshot to `VaultCrdt`.

**Acceptance:** integration tests green (in-memory adapters only — no iroh node, no
real watcher); `make cargo` exits 0; the `VaultSync` struct is constructible from
the IPC command layer.

---

## Step 10 — IPC command: `sync_vault_init`

**What:** Wire `VaultSync` into the Tauri command layer so the UI can trigger
initialization of the sync stack for a project.

```rust
// edge/shell/src/commands.rs (extend invoke_handler!)
#[tauri::command]
async fn sync_vault_init(project_dir: String, state: tauri::State<'_, AppState>)
    -> Result<SyncInitDto, String>;
```

`SyncInitDto` carries `{ node_id: String, status: "ready" | "error" }`.

`AppState` gains a `vault_sync: Option<VaultSync>` field. `sync_vault_init`
constructs a `VaultSync` using `MemorySyncAdapter` + `MemorySnapshotStore` +
`NoopPresence` for now (the live iroh adapters land when real P2P is wired in the
hub, Plan 008).

**Files:**
- `edge/shell/src/commands.rs` (extend `AppState`, add command)
- `edge/host/src/vault/mod.rs` (re-export `VaultSync`, `SyncInitDto`)
- `edge/ui/app/bridge.ts` (add `syncVaultInit(projectDir: string)` helper)
- `edge/ui/store/types.ts` (add `SyncInitDto` interface)
- `edge/host/tests/unit/sync_commands.rs` (new — unit test the DTO mapping)
- `edge/ui/src/__tests__/bridge.test.ts` (extend — `syncVaultInit` invokes
  `"sync_vault_init"`)

**Tests (RED first):**
- Rust: `sync_init_dto_maps_ready_state` — unit-test DTO construction.
- TypeScript: `syncVaultInit_invokes_correct_command` — bridge invokes
  `"sync_vault_init"` with the project dir.

**Acceptance:** command registered; `make cargo` + `make clippy` + `make ts` + `make
verify` exit 0. No regression in the 231 baseline tests.

---

## Dependency order summary

```
1 (deps smoke) → 2 (VaultCrdt skeleton) → 3 (merge logic)
                                        ↓
                                4 (LoroTree folders)
                                        ↓
                        5 (SyncAdapter trait) → 6 (SnapshotStore trait)
                                                        ↓
                                        7 (THE PROJECTOR — do not rush)
                                                        ↓
                                        8 (PresenceBroadcaster trait)
                                                        ↓
                                9 (VaultSync orchestrator)
                                                        ↓
                                10 (IPC: sync_vault_init)
```

---

## Verification

`make verify` exit 0 after every step. End state:

- A `VaultSync` is initialized for a project via `sync_vault_init`.
- Local note saves go through the projector → produce a delta → broadcast via
  `SyncAdapter` (in-memory at this phase; live iroh in Plan 008).
- Incoming deltas apply correctly and are written back to disk (projector, with CAS
  guard and debounce).
- Concurrent edits between in-memory peers converge (deterministic merge test).
- All 231+ cargo tests remain green.

**Plan 008 activates the live iroh adapters and the hub-side authoritative merge.**
Until then, `VaultSync` uses in-memory stubs — the interfaces are correct, the live
wiring lands in the next plan.

---

## Out of scope

- Live iroh-gossip or iroh-docs network (Plan 008 wires the real adapters).
- The hub's authoritative in-memory `LoroDoc` merge (Plan 008).
- iroh-blobs attachment sync (post-Plan 008, deferred).
- UUID→display-name rename rewrite in the `[[wikilink]]` stale-name grace period
  (Obsidian `aliases` field) — projector stub for rename is included; full rewrite
  logic is deferred.
- Presence UI (no frontend presence indicators at this phase).
- Embedding/semantic search over CRDT content.
- Voice (Plan 007, parallel).
- Graph view (Plan 005, precedes this plan).
