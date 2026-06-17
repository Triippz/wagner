//! SnapshotStore trait, in-memory implementation, and iroh-docs HTTP client.

use std::collections::HashMap;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SnapshotStoreError {
    #[error("internal store error")]
    Internal,
}

/// Async interface for persisting and loading Loro CRDT snapshots.
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    /// Persist a snapshot for `note_uuid`. Newer writes supersede older ones.
    async fn save_snapshot(&self, note_uuid: Uuid, snapshot: Vec<u8>) -> Result<(), SnapshotStoreError>;

    /// Load the most recently saved snapshot for `note_uuid`, or `None` if none exists.
    async fn load_snapshot(&self, note_uuid: Uuid) -> Result<Option<Vec<u8>>, SnapshotStoreError>;
}

/// In-memory snapshot store backed by a `HashMap` behind a `tokio::sync::Mutex`.
pub struct InMemorySnapshotStore {
    store: Mutex<HashMap<Uuid, Vec<u8>>>,
}

impl InMemorySnapshotStore {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SnapshotStore for InMemorySnapshotStore {
    async fn save_snapshot(&self, note_uuid: Uuid, snapshot: Vec<u8>) -> Result<(), SnapshotStoreError> {
        self.store.lock().await.insert(note_uuid, snapshot);
        Ok(())
    }

    async fn load_snapshot(&self, note_uuid: Uuid) -> Result<Option<Vec<u8>>, SnapshotStoreError> {
        Ok(self.store.lock().await.get(&note_uuid).cloned())
    }
}

// ---------------------------------------------------------------------------
// Live iroh-docs HTTP client (Plan 008 Step 7)
// ---------------------------------------------------------------------------

/// HTTP client wrapping the local iroh daemon's iroh-docs REST API.
///
/// Stores base64-encoded loro snapshots keyed by note UUID, using the iroh daemon
/// as a durable catch-up store. The iroh daemon is expected to run as a sidecar
/// (see ADR-0003). Not used in hermetic tests — inject `InMemorySnapshotStore` there.
///
/// ponytail: HTTP sidecar API is sufficient for catch-up; a native iroh-docs
/// Rust integration is a future optimisation (out of scope for Plan 008).
pub struct IrohDocsStore {
    client: reqwest::Client,
    base_url: String,
    namespace: String,
}

impl IrohDocsStore {
    /// Create a new store client.
    /// `base_url` — e.g. `"http://127.0.0.1:4919"` (iroh daemon HTTP port).
    /// `namespace` — iroh-docs namespace ID for vault snapshots.
    pub fn new(base_url: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            namespace: namespace.into(),
        }
    }

    fn url(&self, note_uuid: Uuid) -> String {
        format!(
            "{}/docs/{}/entries/{}",
            self.base_url, self.namespace, note_uuid
        )
    }
}

#[async_trait]
impl SnapshotStore for IrohDocsStore {
    async fn save_snapshot(&self, note_uuid: Uuid, snapshot: Vec<u8>) -> Result<(), SnapshotStoreError> {
        self.client
            .put(self.url(note_uuid))
            .body(snapshot)
            .send()
            .await
            .map_err(|_| SnapshotStoreError::Internal)?;
        Ok(())
    }

    async fn load_snapshot(&self, note_uuid: Uuid) -> Result<Option<Vec<u8>>, SnapshotStoreError> {
        let resp = self
            .client
            .get(self.url(note_uuid))
            .send()
            .await
            .map_err(|_| SnapshotStoreError::Internal)?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let bytes = resp.bytes().await.map_err(|_| SnapshotStoreError::Internal)?;
        Ok(Some(bytes.to_vec()))
    }
}
