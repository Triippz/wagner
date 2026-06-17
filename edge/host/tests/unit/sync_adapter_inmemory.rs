//! InMemorySyncAdapter tests.

use uuid::Uuid;
use wagner_edge_host::vault::sync_adapter::{InMemorySyncAdapter, SyncAdapter};

#[tokio::test]
async fn test_broadcast_and_receive_delta() {
    let adapter = InMemorySyncAdapter::new();
    let id = Uuid::new_v4();

    adapter.subscribe(id).await.unwrap();
    adapter.broadcast_delta(id, vec![1, 2, 3]).await.unwrap();

    let deltas = adapter.pending_deltas(id).await.unwrap();
    assert_eq!(deltas, vec![vec![1u8, 2, 3]], "must receive broadcast delta");
}

#[tokio::test]
async fn test_pending_deltas_empty_before_broadcast() {
    let adapter = InMemorySyncAdapter::new();
    let id = Uuid::new_v4();

    adapter.subscribe(id).await.unwrap();
    let deltas = adapter.pending_deltas(id).await.unwrap();
    assert!(deltas.is_empty(), "no deltas before any broadcast");
}
