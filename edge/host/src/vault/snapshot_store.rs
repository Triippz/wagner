//! SnapshotStore trait and in-memory implementation.

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
