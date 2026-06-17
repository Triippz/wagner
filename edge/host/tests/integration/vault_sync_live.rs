//! Live iroh-gossip integration test (Plan 008 Step 7).
//!
//! Two in-process iroh Endpoints exchange a loro delta via GossipSyncAdapter.
//! No relay, no DNS — uses MemoryLookup for loopback address distribution.
//!
//! Requires OS networking (loopback). Marked #[ignore] by default so it does not
//! run in hermetic CI. Run with:
//!   cargo test --test vault_sync_live -- --ignored

use std::time::Duration;

use bytes::Bytes;
use iroh::{
    Endpoint, RelayMode,
    address_lookup::memory::MemoryLookup,
    endpoint::presets,
};
use iroh_gossip::{
    api::Event,
    net::{Gossip, GOSSIP_ALPN},
    proto::TopicId,
};
use futures::TryStreamExt;
use tokio::time::timeout;
use uuid::Uuid;
use wagner_edge_host::vault::sync_adapter::{GossipSyncAdapter, SyncAdapter};

/// Derive the same TopicId that GossipSyncAdapter uses internally.
fn topic_for(note_uuid: Uuid) -> TopicId {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(note_uuid.as_bytes());
    TopicId::from_bytes(bytes)
}

#[tokio::test]
#[ignore = "requires OS networking (loopback); run with --ignored"]
async fn two_iroh_peers_exchange_loro_delta_in_memory() {
    // ---- Build peer A ----
    let lookup_a = MemoryLookup::new();
    let ep_a = Endpoint::builder(presets::N0)
        .address_lookup(lookup_a.clone())
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await
        .expect("ep_a bind");
    let gossip_a = Gossip::builder().spawn(ep_a.clone());

    // ---- Build peer B ----
    let lookup_b = MemoryLookup::new();
    let ep_b = Endpoint::builder(presets::N0)
        .address_lookup(lookup_b.clone())
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await
        .expect("ep_b bind");
    let gossip_b = Gossip::builder().spawn(ep_b.clone());

    // Set up router for both endpoints to accept gossip ALPN connections.
    let router_a = iroh::protocol::Router::builder(ep_a.clone())
        .accept(GOSSIP_ALPN, gossip_a.clone())
        .spawn();
    let router_b = iroh::protocol::Router::builder(ep_b.clone())
        .accept(GOSSIP_ALPN, gossip_b.clone())
        .spawn();

    // Exchange addresses: A's addr → B's lookup; B's addr → A's lookup.
    let addr_a = ep_a.addr();
    let addr_b = ep_b.addr();
    lookup_a.add_endpoint_info(addr_b.clone());
    lookup_b.add_endpoint_info(addr_a.clone());

    let note_uid = Uuid::new_v4();
    let topic = topic_for(note_uid);

    // B subscribes first with no bootstrap (it waits for A to connect).
    let topic_b = gossip_b
        .subscribe(topic, vec![])
        .await
        .expect("gossip_b subscribe");
    let (_, mut receiver_b) = topic_b.split();

    // A subscribes, bootstrapping to B's endpoint id.
    let mut topic_a = gossip_a
        .subscribe_and_join(topic, vec![ep_b.id()])
        .await
        .expect("gossip_a subscribe_and_join");

    // Broadcast a "loro delta" from A.
    let delta: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
    topic_a
        .broadcast(Bytes::from(delta.clone()))
        .await
        .expect("broadcast");

    // B should receive it within 2 seconds.
    let received = timeout(Duration::from_secs(2), async {
        loop {
            match receiver_b.try_next().await.expect("recv stream") {
                Some(Event::Received(msg)) => return msg.content.to_vec(),
                Some(_) => continue, // NeighborUp or other events
                None => panic!("gossip_b stream closed"),
            }
        }
    })
    .await
    .expect("timed out waiting for delta");

    assert_eq!(received, delta, "received bytes must match sent loro delta");

    // ---- GossipSyncAdapter wrapper test ----
    // Verify that GossipSyncAdapter.subscribe + broadcast_delta compiles and runs.
    let adapter_a = GossipSyncAdapter::new(gossip_a.clone());
    adapter_a.subscribe(note_uid).await.expect("adapter subscribe");
    adapter_a
        .broadcast_delta(note_uid, vec![1, 2, 3])
        .await
        .expect("adapter broadcast");

    // Shutdown.
    router_a.shutdown().await.ok();
    router_b.shutdown().await.ok();
}
