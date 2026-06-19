//! 011 P1 — the in-process bus (`bus::Bus`). Built standalone, not yet wired into
//! the app. Covers the plan Step 1 test list: publish→subscribe delivery,
//! per-stream `seq` (monotonic + per-stream independent), topic/namespace
//! filtering, the slow-subscriber `Lagged` path + recovery, and many concurrent
//! publishers/subscribers.

use std::sync::Arc;

use wagner_edge_host::bus::{
    Bus, Envelope, Event, EventId, NodeId, ParticipantId, ParticipantKind, RecvError, RunEvent,
    Scope, StreamId, Subscription, Timestamp, VaultEvent,
};

fn origin() -> ParticipantId {
    ParticipantId {
        node: NodeId("test-node".into()),
        kind: ParticipantKind::System,
        name: "test".into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    }
}

fn envelope(stream: StreamId, payload: Event) -> Envelope {
    Envelope::new(
        EventId("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()),
        Timestamp("2026-06-19T00:00:00Z".into()),
        origin(),
        stream,
        0, // bus reassigns the authoritative per-stream seq
        Scope { user: "u".into(), workspace: "w".into() },
        payload,
    )
}

fn run_finished(id: &str) -> Event {
    Event::Run(RunEvent::Finished { run_id: id.into(), ok: true })
}

fn vault_updated(path: &str) -> Event {
    Event::Vault(VaultEvent::NoteUpdated { path: path.into(), rev: 1 })
}

#[tokio::test]
async fn publish_then_subscribe_delivers() {
    let bus = Bus::new(16);
    let mut sub = bus.subscribe(Subscription { topic: "*".into(), filter: None });
    bus.publish(envelope(StreamId::Run("r1".into()), run_finished("r1")));
    let got = sub.recv().await.expect("delivery");
    assert_eq!(got.payload, run_finished("r1"));
}

#[tokio::test]
async fn per_stream_seq_is_monotonic_and_independent() {
    let bus = Bus::new(64);
    let mut sub = bus.subscribe(Subscription { topic: "*".into(), filter: None });

    // Two publishes on stream A, one on stream B.
    let a0 = bus.publish(envelope(StreamId::Run("A".into()), run_finished("A")));
    let a1 = bus.publish(envelope(StreamId::Run("A".into()), run_finished("A")));
    let b0 = bus.publish(envelope(StreamId::Run("B".into()), run_finished("B")));

    // Publish reassigns the authoritative per-stream seq: 0,1 on A; 0 on B.
    assert_eq!(a0.seq, 0);
    assert_eq!(a1.seq, 1);
    assert_eq!(b0.seq, 0, "a new stream's seq is independent");

    // The fanned-out envelopes carry the stamped seq, in publish order.
    assert_eq!(sub.recv().await.unwrap().seq, 0);
    assert_eq!(sub.recv().await.unwrap().seq, 1);
    assert_eq!(sub.recv().await.unwrap().seq, 0);
}

#[tokio::test]
async fn subscription_filters_by_namespace() {
    let bus = Bus::new(16);
    let mut vault_sub = bus.subscribe(Subscription { topic: "vault".into(), filter: None });

    bus.publish(envelope(StreamId::Run("r".into()), run_finished("r"))); // filtered out
    bus.publish(envelope(StreamId::Workspace("w".into()), vault_updated("n.md"))); // kept

    let got = vault_sub.recv().await.expect("vault event delivered");
    assert_eq!(got.payload, vault_updated("n.md"));
}

#[tokio::test]
async fn subscription_filters_by_stream() {
    let bus = Bus::new(16);
    let mut sub = bus.subscribe(Subscription { topic: "stream".into(), filter: Some("A".into()) });

    bus.publish(envelope(StreamId::Run("B".into()), run_finished("B"))); // wrong stream
    bus.publish(envelope(StreamId::Run("A".into()), run_finished("A"))); // match

    let got = sub.recv().await.expect("stream A event");
    assert_eq!(got.stream, StreamId::Run("A".into()));
}

#[tokio::test]
async fn slow_subscriber_lags_then_recovers() {
    // Capacity 2; publish 4 without receiving → the oldest 2 are dropped.
    let bus = Bus::new(2);
    let mut sub = bus.subscribe(Subscription { topic: "*".into(), filter: None });
    for _ in 0..4 {
        bus.publish(envelope(StreamId::Run("r".into()), run_finished("r")));
    }
    match sub.recv().await {
        Err(RecvError::Lagged(n)) => assert!(n >= 1, "reports how many were dropped"),
        other => panic!("expected Lagged, got {other:?}"),
    }
    // After surfacing the lag, the subscriber recovers and keeps delivering.
    assert!(sub.recv().await.is_ok(), "recovers after Lagged");
}

#[tokio::test]
async fn many_concurrent_publishers_and_subscribers() {
    let bus = Arc::new(Bus::new(1024));
    const PUBLISHERS: usize = 4;
    const PER_PUBLISHER: usize = 25;
    const SUBSCRIBERS: usize = 3;
    let total = PUBLISHERS * PER_PUBLISHER;

    let mut consumers = Vec::new();
    for _ in 0..SUBSCRIBERS {
        let mut sub = bus.subscribe(Subscription { topic: "*".into(), filter: None });
        consumers.push(tokio::spawn(async move {
            let mut seen = 0;
            while seen < total {
                match sub.recv().await {
                    Ok(_) => seen += 1,
                    Err(RecvError::Lagged(_)) => panic!("capacity should be ample"),
                    Err(RecvError::Closed) => break,
                }
            }
            seen
        }));
    }

    let mut producers = Vec::new();
    for p in 0..PUBLISHERS {
        let bus = Arc::clone(&bus);
        producers.push(tokio::spawn(async move {
            for _ in 0..PER_PUBLISHER {
                bus.publish(envelope(StreamId::Run(format!("p{p}")), run_finished("x")));
            }
        }));
    }

    for prod in producers {
        prod.await.unwrap();
    }
    for cons in consumers {
        assert_eq!(cons.await.unwrap(), total, "every subscriber sees every envelope");
    }
}
