//! T024 — No-self-arm / refused-attach (FR-212, SC-203, US2-AS-3/4, EC-003/004).
//!
//! Attach is refused unless the host is ARMED, the requester is VERIFIED, and the
//! requester OWNS the host (same operator). A local run is unaffected by a
//! refused attach.

use wagner_edge_host::remote::arm::{arm, LocalOperator};
use wagner_edge_host::remote::session::{attach, AttachRefusal, VerifiedRemote};

fn armed_state() -> wagner_edge_host::remote::arm::ArmState {
    arm(&LocalOperator::from_host("op-1"), "node", "tkt", 0, 60_000).0
}

#[test]
fn attach_to_a_never_armed_host_is_refused() {
    let disarmed = wagner_edge_host::remote::arm::ArmState::disarmed();
    let who = VerifiedRemote::new("op-1", "cli-1");
    let r = attach(&disarmed, &who, 1_000);
    assert_eq!(r.unwrap_err(), AttachRefusal::NotArmed);
}

#[test]
fn attach_by_a_non_owner_is_refused() {
    let who = VerifiedRemote::new("op-2", "cli-1"); // different operator
    let r = attach(&armed_state(), &who, 1_000);
    assert_eq!(r.unwrap_err(), AttachRefusal::NotOwner);
}

#[test]
fn attach_after_expiry_is_refused() {
    let who = VerifiedRemote::new("op-1", "cli-1");
    // armed_state expires at 60_000; attaching past that lapses.
    let r = attach(&armed_state(), &who, 60_001);
    assert_eq!(r.unwrap_err(), AttachRefusal::Expired);
}

#[test]
fn owner_verified_and_armed_attaches() {
    let who = VerifiedRemote::new("op-1", "cli-1");
    let session = attach(&armed_state(), &who, 1_000).expect("owner attach must succeed");
    assert_eq!(session.client_id(), "cli-1");
    assert_eq!(session.operator_id(), "op-1");
}
