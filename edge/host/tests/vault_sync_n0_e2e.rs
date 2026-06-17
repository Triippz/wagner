//! Two-peer end-to-end sync test over n0's public relay infrastructure.
//!
//! Peer A and peer B are real iroh Endpoints configured with `RelayMode::Default`
//! (the n0-hosted production relays). A MemoryLookup is added alongside the n0 DNS
//! preset so peers can find each other instantly without waiting for DNS propagation,
//! while the actual gossip traffic still routes through the n0 relay.
//!
//! Scenario:
//!   1. Peer A creates a loro note with text "hello from A".
//!   2. A exports a loro snapshot and broadcasts it via GossipSyncAdapter.
//!   3. Peer B receives the bytes on the gossip stream.
//!   4. B imports the snapshot into its own VaultCrdt.
//!   5. Assertion: B's document contains A's text.
//!
//! Marked #[ignore] — requires live internet (n0 relays). Run with:
//!   cargo test -p wagner-edge-host --test vault_sync_n0_e2e -- --ignored --nocapture
//!
//! Or via the Makefile: `make sync-e2e`

use std::time::Duration;

use bytes::Bytes;
use futures::TryStreamExt;
use iroh::{Endpoint, address_lookup::memory::MemoryLookup, endpoint::presets};
use iroh_gossip::{
    api::Event,
    net::{Gossip, GOSSIP_ALPN},
    proto::TopicId,
};
use loro::LoroDoc;
use tokio::time::timeout;
use uuid::Uuid;
use wagner_edge_host::vault::{
    sync_adapter::{GossipSyncAdapter, SyncAdapter, vault_relay_mode},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive the same TopicId that GossipSyncAdapter uses internally.
fn topic_for(note_uuid: Uuid) -> TopicId {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(note_uuid.as_bytes());
    TopicId::from_bytes(bytes)
}

/// Build an iroh Endpoint using n0's relay infrastructure + an in-process MemoryLookup.
///
/// Using `presets::N0` sets up DNS discovery and RelayMode::Default (n0 relays).
/// The MemoryLookup is added as an ADDITIONAL lookup so we can register peer
/// addresses in-process, avoiding DNS propagation delay while still routing
/// gossip traffic through the real n0 relay.
///
/// n0 relays for now; swap to our own relay URLs later (change vault_relay_mode()).
async fn build_n0_endpoint(lookup: MemoryLookup) -> Endpoint {
    // presets::N0 already sets RelayMode::Default; we call vault_relay_mode() to
    // make the config seam explicit — both resolve to RelayMode::Default today.
    Endpoint::builder(presets::N0)
        .relay_mode(vault_relay_mode()) // n0 relays; one-line swap when we have our own
        .address_lookup(lookup)
        .bind()
        .await
        .expect("iroh endpoint bind with n0 relays")
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires live internet (n0 relays); run with: make sync-e2e"]
async fn two_peers_sync_loro_note_over_n0_relay() {
    // Allow up to 30 s total for relay path establishment + delta delivery.
    let test_body = async {
        let note_uuid = Uuid::new_v4();
        let topic = topic_for(note_uuid);

        // ---- Peer A ----
        let lookup_a = MemoryLookup::new();
        let ep_a = build_n0_endpoint(lookup_a.clone()).await;
        let gossip_a = Gossip::builder().spawn(ep_a.clone());
        let router_a = iroh::protocol::Router::builder(ep_a.clone())
            .accept(GOSSIP_ALPN, gossip_a.clone())
            .spawn();

        // ---- Peer B ----
        let lookup_b = MemoryLookup::new();
        let ep_b = build_n0_endpoint(lookup_b.clone()).await;
        let gossip_b = Gossip::builder().spawn(ep_b.clone());
        let router_b = iroh::protocol::Router::builder(ep_b.clone())
            .accept(GOSSIP_ALPN, gossip_b.clone())
            .spawn();

        // Exchange endpoint addresses in-process (fast, no DNS wait).
        // ep.addr() includes the relay URL so iroh can route through n0 relay
        // even when a direct path is not (yet) available.
        let addr_a = ep_a.addr();
        let addr_b = ep_b.addr();
        lookup_a.add_endpoint_info(addr_b.clone());
        lookup_b.add_endpoint_info(addr_a.clone());

        println!("[e2e] peer A id: {}", ep_a.id().fmt_short());
        println!("[e2e] peer B id: {}", ep_b.id().fmt_short());
        println!("[e2e] addr_a relay: {:?}", addr_a.relay_urls().next());
        println!("[e2e] addr_b relay: {:?}", addr_b.relay_urls().next());

        // B subscribes first with no bootstrap.
        let topic_b = gossip_b
            .subscribe(topic, vec![])
            .await
            .expect("gossip_b subscribe");
        let (_, mut receiver_b) = topic_b.split();

        // A subscribes and joins, bootstrapping to B.
        // subscribe_and_join waits until at least one peer (B) is reachable.
        let mut topic_a = gossip_a
            .subscribe_and_join(topic, vec![ep_b.id()])
            .await
            .expect("gossip_a subscribe_and_join — n0 relay reachable");

        // ---- Build a loro note on A ----
        let loro_a = LoroDoc::new();
        let text = loro_a.get_text("content");
        text.insert(0, "hello from A").expect("loro text insert A");
        loro_a.commit();
        let snapshot_bytes = loro_a
            .export(loro::ExportMode::Snapshot)
            .expect("loro export A");

        println!("[e2e] A broadcasting {} bytes", snapshot_bytes.len());

        // A broadcasts via GossipSyncAdapter (exercises the real adapter code path).
        let adapter_a = GossipSyncAdapter::new(gossip_a.clone());
        adapter_a
            .subscribe(note_uuid)
            .await
            .expect("adapter_a subscribe");
        adapter_a
            .broadcast_delta(note_uuid, snapshot_bytes.clone())
            .await
            .expect("adapter_a broadcast_delta");

        // Also broadcast directly on the topic handle to cover the raw gossip path.
        topic_a
            .broadcast(Bytes::from(snapshot_bytes.clone()))
            .await
            .expect("topic_a broadcast");

        // ---- B receives and merges ----
        let received_bytes = timeout(Duration::from_secs(30), async {
            loop {
                match receiver_b.try_next().await.expect("recv stream") {
                    Some(Event::Received(msg)) => return msg.content.to_vec(),
                    Some(_) => continue, // NeighborUp / NeighborDown events
                    None => panic!("gossip_b stream closed before receiving delta"),
                }
            }
        })
        .await
        .expect("timed out waiting for delta from n0 relay (30 s)");

        println!("[e2e] B received {} bytes", received_bytes.len());

        // B imports into a fresh loro doc and asserts the text converges.
        let loro_b = LoroDoc::new();
        loro_b
            .import(&received_bytes)
            .expect("loro import B — CRDT merge");
        let text_b = loro_b.get_text("content");
        let content_b = text_b.to_string();

        println!("[e2e] B content: {:?}", content_b);
        assert_eq!(
            content_b, "hello from A",
            "peer B's VaultCrdt must converge to A's note content after n0-relay sync"
        );

        // ---- Cleanup ----
        router_a.shutdown().await.ok();
        router_b.shutdown().await.ok();
    };

    timeout(Duration::from_secs(30), test_body)
        .await
        .expect("e2e test timed out after 30 s — n0 relay unreachable or too slow");
}
