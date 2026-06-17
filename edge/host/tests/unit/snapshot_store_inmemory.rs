//! InMemorySnapshotStore tests.

use uuid::Uuid;
use wagner_edge_host::vault::snapshot_store::{InMemorySnapshotStore, SnapshotStore};

#[tokio::test]
async fn test_load_returns_none_before_save() {
    let store = InMemorySnapshotStore::new();
    let id = Uuid::new_v4();
    let result = store.load_snapshot(id).await.unwrap();
    assert!(result.is_none(), "load before save must return None");
}

#[tokio::test]
async fn test_save_and_load_roundtrip() {
    let store = InMemorySnapshotStore::new();
    let id = Uuid::new_v4();
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

    store.save_snapshot(id, data.clone()).await.unwrap();
    let loaded = store.load_snapshot(id).await.unwrap();
    assert_eq!(loaded, Some(data), "loaded snapshot must match saved data");
}

#[tokio::test]
async fn test_second_save_overwrites_first() {
    let store = InMemorySnapshotStore::new();
    let id = Uuid::new_v4();

    store.save_snapshot(id, vec![1, 2, 3]).await.unwrap();
    store.save_snapshot(id, vec![4, 5, 6]).await.unwrap();

    let loaded = store.load_snapshot(id).await.unwrap();
    assert_eq!(loaded, Some(vec![4u8, 5, 6]), "second save must overwrite first");
}
