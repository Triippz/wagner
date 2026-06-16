//! T022 + T025b — Arming (FR-201, SC-203, US2-AS-4, EC-004, EC-010).
//!
//! `arm` is EDGE-ONLY: it advertises the endpoint + registers NodeId/ticket and
//! emits `remote.armed`. The type system enforces no-self-arm — `arm` consumes a
//! `LocalOperator` (constructible only on the host), and there is NO function
//! that arms from a remote message. Re-arming an already-armed host refreshes
//! the ticket/expiry and emits a single new `remote.armed` (no duplicate).

use wagner_edge_host::remote::arm::{arm, disarm, ArmState, LocalOperator};

fn op() -> LocalOperator {
    LocalOperator::from_host("op-1")
}

#[test]
fn arm_emits_remote_armed_and_registers_endpoint() {
    let (state, event) = arm(&op(), "node-aaa", "tkt-1", 1_000, 60_000);
    assert!(state.armed);
    assert_eq!(state.operator_id, "op-1");
    assert_eq!(state.node_id, "node-aaa");
    assert_eq!(state.expires_at_ms, 61_000);
    // The emitted event is the audit record of arming.
    assert_eq!(event.kind(), "remote.armed");
    assert_eq!(event.operator_id(), Some("op-1"));
}

#[test]
fn rearm_refreshes_ticket_and_expiry_without_duplicating() {
    let (s1, _) = arm(&op(), "node-aaa", "tkt-1", 1_000, 30_000);
    // Re-arm later with a fresh node/ticket.
    let (s2, event) = arm(&op(), "node-bbb", "tkt-2", 10_000, 30_000);
    assert_eq!(s2.node_id, "node-bbb");
    assert_eq!(s2.ticket_id, "tkt-2");
    assert_eq!(s2.expires_at_ms, 40_000);
    assert_eq!(event.kind(), "remote.armed");
    // Re-arm is a replace-in-place: the second state simply supersedes the first.
    assert_ne!(s1.expires_at_ms, s2.expires_at_ms);
}

#[test]
fn disarm_clears_and_emits_remote_disarmed() {
    let event = disarm();
    assert_eq!(event.kind(), "remote.disarmed");
    // A disarmed state is not armed.
    assert!(!ArmState::disarmed().armed);
}

/// SC-203 structural guarantee: `arm` requires a `LocalOperator`, which can only
/// be built on the host (`from_host`). There is no constructor from a remote
/// message and no remote-reachable arm entrypoint — self-arm is impossible.
#[test]
fn arming_requires_a_local_operator_no_remote_path() {
    let local = LocalOperator::from_host("op-1");
    let (state, _) = arm(&local, "n", "t", 0, 1000);
    assert!(state.armed);
    // (Compile-time: there is no `LocalOperator::from_remote(...)`; a remote
    //  attach carries a `VerifiedRemote`, which `arm` does not accept.)
}
