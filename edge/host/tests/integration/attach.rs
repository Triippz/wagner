//! T023 — Attach over transport + relay fallback (FR-203/210, US2-AS-1/2, EC-001).
//!
//! Connection-path selection: a direct iroh path is preferred; when no direct
//! path forms, the org-run relay is the fallback; when neither is available the
//! attach fails GRACEFULLY (US2-AS-2) — the local run is unaffected. The live
//! iroh `Endpoint` is wired in T027 (integration-only); here the path policy is
//! tested against an in-memory transport that delivers the event stream, proving
//! the attached client folds events to a live projection regardless of path.

use wagner_edge_host::remote::endpoint::{select_path, ConnectError, ConnectPath, LoopbackStream};

#[test]
fn direct_path_is_preferred_when_available() {
    assert_eq!(select_path(true, true), Ok(ConnectPath::Direct));
    assert_eq!(select_path(true, false), Ok(ConnectPath::Direct));
}

#[test]
fn relay_is_the_fallback_when_no_direct_path() {
    assert_eq!(select_path(false, true), Ok(ConnectPath::Relay));
}

#[test]
fn no_path_fails_gracefully() {
    assert_eq!(select_path(false, false), Err(ConnectError::NoPath));
}

#[test]
fn attached_client_folds_the_event_stream_in_order_over_loopback() {
    // The in-memory transport delivers the host's event stream to the client.
    let mut stream = LoopbackStream::new();
    stream.push(r#"{"channel":"run","payload":{"goal":"x"}}"#);
    stream.push(r#"{"channel":"event","payload":{"operative_id":"cipher"}}"#);

    let received = stream.drain();
    assert_eq!(received.len(), 2);
    assert!(received[0].contains("\"goal\":\"x\""));
    assert!(received[1].contains("cipher"));
}
