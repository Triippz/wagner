//! SyncAdapter trait, in-memory implementation, and iroh-gossip live adapter.

use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Bytes;
use iroh::RelayMode;
use iroh_gossip::{api::GossipSender, net::Gossip, proto::TopicId};
use thiserror::Error;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Relay configuration seam
// ---------------------------------------------------------------------------

/// Returns the relay mode used when building iroh `Endpoint`s for vault gossip sync.
///
/// Currently: `RelayMode::Default` — the n0-hosted production relay infrastructure.
/// Swap this to `RelayMode::Custom(our_relay_map)` once we operate our own relay.
///
/// n0 relays for now; swap to our own relay URLs later.
///
/// # Example
/// ```no_run
/// use iroh::{Endpoint, endpoint::presets};
/// use wagner_edge_host::vault::sync_adapter::vault_relay_mode;
///
/// # #[tokio::main]
/// # async fn main() {
/// let ep = Endpoint::builder(presets::N0)
///     .relay_mode(vault_relay_mode())
///     .bind()
///     .await
///     .expect("bind");
/// # }
/// ```
pub fn vault_relay_mode() -> RelayMode {
    // n0 relays for now; swap to our own relay URLs later (one-line change here).
    RelayMode::Default
}

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

// ---------------------------------------------------------------------------
// Live iroh-gossip adapter (Plan 008 Step 7)
// ---------------------------------------------------------------------------

/// Topic-keyed gossip sender map. One per subscribed note UUID.
type TopicSenders = Mutex<HashMap<Uuid, GossipSender>>;

/// Live iroh-gossip `SyncAdapter`. One Gossip handle shared across all note topics.
///
/// Each `subscribe` call joins a per-note gossip topic derived from the UUID bytes.
/// `broadcast_delta` broadcasts the delta bytes to all peers in the topic.
/// `pending_deltas` is not supported for the live adapter — use a dedicated
/// receiver task instead (see the integration test for the subscribe_and_join pattern).
///
/// The iroh `Endpoint` backing the `Gossip` handle should be built with
/// [`vault_relay_mode()`] so it uses the n0 relay infrastructure (default) or
/// our own relay once we operate one.
///
/// ponytail: pending_deltas returns empty for live adapter — callers should use
/// the gossip receiver stream directly for production ingest.
pub struct GossipSyncAdapter {
    gossip: Gossip,
    senders: TopicSenders,
}

impl GossipSyncAdapter {
    /// Create a new adapter backed by the given iroh-gossip handle.
    pub fn new(gossip: Gossip) -> Self {
        Self {
            gossip,
            senders: Mutex::new(HashMap::new()),
        }
    }

    /// Derive a deterministic `TopicId` from a note UUID.
    fn topic_for(note_uuid: Uuid) -> TopicId {
        // UUID is 16 bytes; TopicId is 32 bytes — pad with zeros.
        // ponytail: SHA-256 would be more collision-resistant but UUID is already
        // unique by construction; this is sufficient for the gossip namespace.
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(note_uuid.as_bytes());
        TopicId::from_bytes(bytes)
    }
}

#[async_trait]
impl SyncAdapter for GossipSyncAdapter {
    async fn subscribe(&self, note_uuid: Uuid) -> Result<(), SyncAdapterError> {
        let topic = Self::topic_for(note_uuid);
        // Subscribe with no bootstrap peers — the hub or other peers join us.
        let topic_handle = self
            .gossip
            .subscribe(topic, vec![])
            .await
            .map_err(|_| SyncAdapterError::SendFailed)?;
        let (sender, _receiver) = topic_handle.split();
        self.senders.lock().await.insert(note_uuid, sender);
        Ok(())
    }

    async fn broadcast_delta(&self, note_uuid: Uuid, delta: Vec<u8>) -> Result<(), SyncAdapterError> {
        let senders = self.senders.lock().await;
        let sender = senders
            .get(&note_uuid)
            .ok_or(SyncAdapterError::NotSubscribed(note_uuid))?;
        sender
            .broadcast(Bytes::from(delta))
            .await
            .map_err(|_| SyncAdapterError::SendFailed)?;
        Ok(())
    }

    /// Not supported for the live adapter; returns empty (use the GossipReceiver stream).
    async fn pending_deltas(&self, _note_uuid: Uuid) -> Result<Vec<Vec<u8>>, SyncAdapterError> {
        Ok(vec![])
    }
}
