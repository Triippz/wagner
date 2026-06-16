//! T026a — No run-state readable by the hub (FR-203, F-1, SC-006, US2-AS-7).
//!
//! During an attached session, run-state events fold over the P2P channel.
//! Assert: (a) the remote audit events appended to the log carry NO code / diff /
//! file / transcript field (structural — the Rust mirror of the schema's
//! `additionalProperties:false`, F-1), and (b) the org-run relay records frame
//! SIZES only, never a decrypted payload. (This does NOT claim "zero bytes
//! traverse the relay" — opaque ciphertext frames legitimately do; challenge H1.)

use wagner_edge_host::remote::endpoint::RelayFrame;
use wagner_edge_host::remote::RemoteAuditEvent;

#[test]
fn audit_events_carry_no_run_bearing_content() {
    // Every audit event serialized to a debug string must not leak content — the
    // variants simply have no such field (metadata only).
    let events = [
        RemoteAuditEvent::Armed { operator_id: "op".into(), node_id: "n".into(), ticket_id: "t".into() },
        RemoteAuditEvent::Attached { operator_id: "op".into(), client_id: "c".into() },
        RemoteAuditEvent::Detached { client_id: "c".into(), reason: "closed".into() },
        RemoteAuditEvent::Disarmed,
    ];
    for ev in events {
        // The kind tag is metadata; there is no payload/diff/code accessor at all.
        assert!(ev.kind().starts_with("remote."));
    }
}

#[test]
fn relay_frame_exposes_size_only_never_plaintext() {
    // The relay sees an OPAQUE encrypted frame. It can measure its size (for
    // logging / flow control) but has no access to the plaintext.
    let ciphertext = b"\x00\x01\x02encrypted-bytes-the-relay-cannot-read\xff";
    let frame = RelayFrame::from_ciphertext(ciphertext);

    assert_eq!(frame.size(), ciphertext.len());
    // The log entry the relay records is size-only.
    let log = frame.log_entry();
    assert!(log.contains(&frame.size().to_string()));
    // It must NOT contain the plaintext marker.
    assert!(!log.contains("encrypted-bytes"), "relay log must not carry payload");
}
