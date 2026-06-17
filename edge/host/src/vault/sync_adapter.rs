//! SyncAdapter trait and in-memory implementation.

use std::collections::HashMap;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

const BROADCAST_CAPACITY: usize = 256;

type NoteChannel = (broadcast::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>);

#[derive(Debug, Error)]
pub enum SyncAdapterError {
    #[error("topic not subscribed: {0}")]
    NotSubscribed(Uuid),
    #[error("broadcast send failed")]
    SendFailed,
}

/// Async transport interface for per-note delta propagation.
#[async_trait]
pub trait SyncAdapter: Send + Sync {
    /// Subscribe to delta updates for `note_uuid`.
    async fn subscribe(&self, note_uuid: Uuid) -> Result<(), SyncAdapterError>;

    /// Broadcast a delta for `note_uuid` to all subscribers.
    async fn broadcast_delta(&self, note_uuid: Uuid, delta: Vec<u8>) -> Result<(), SyncAdapterError>;

    /// Drain all pending deltas for `note_uuid`.
    async fn pending_deltas(&self, note_uuid: Uuid) -> Result<Vec<Vec<u8>>, SyncAdapterError>;
}

/// In-memory sync adapter backed by `tokio::sync::broadcast`.
pub struct InMemorySyncAdapter {
    channels: Mutex<HashMap<Uuid, NoteChannel>>,
}

impl InMemorySyncAdapter {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySyncAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SyncAdapter for InMemorySyncAdapter {
    async fn subscribe(&self, note_uuid: Uuid) -> Result<(), SyncAdapterError> {
        let mut map = self.channels.lock().await;
        map.entry(note_uuid).or_insert_with(|| {
            let (tx, rx) = broadcast::channel(BROADCAST_CAPACITY);
            (tx, rx)
        });
        Ok(())
    }

    async fn broadcast_delta(&self, note_uuid: Uuid, delta: Vec<u8>) -> Result<(), SyncAdapterError> {
        let map = self.channels.lock().await;
        let (tx, _) = map.get(&note_uuid).ok_or(SyncAdapterError::NotSubscribed(note_uuid))?;
        tx.send(delta).map_err(|_| SyncAdapterError::SendFailed)?;
        Ok(())
    }

    async fn pending_deltas(&self, note_uuid: Uuid) -> Result<Vec<Vec<u8>>, SyncAdapterError> {
        let mut map = self.channels.lock().await;
        let (_, rx) = map.get_mut(&note_uuid).ok_or(SyncAdapterError::NotSubscribed(note_uuid))?;
        let mut deltas = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(delta) => deltas.push(delta),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => break,
                Err(broadcast::error::TryRecvError::Lagged(_)) => break,
            }
        }
        Ok(deltas)
    }
}
