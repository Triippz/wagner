//! T037 — Concurrent control: first-write-wins (CL-204, EC-005).
//!
//! Two attached clients send conflicting permission answers for the SAME prompt.
//! The gate serializes: the first answer through the seam wins, the later one is
//! a no-op (the transmission is already resolved + removed).

use wagner_edge_host::remote::control::{route_control, ControlAction, ControlOrigin};
use wagner_edge_host::transmissions::{Decision, TransmissionRegistry};

fn client(id: &str) -> ControlOrigin {
    ControlOrigin::Remote { client_id: id.into(), operator_id: "op-1".into() }
}

#[tokio::test]
async fn first_answer_wins_second_is_a_noop() {
    let reg = TransmissionRegistry::default();
    let rx = reg.open("tx-1");

    // Client A answers ALLOW first.
    let a = route_control(
        &reg,
        &client("cli-A"),
        &ControlAction::AnswerPermission { transmission_id: "tx-1".into(), answer: "allow".into() },
    );
    // Client B answers DENY a moment later for the same prompt.
    let b = route_control(
        &reg,
        &client("cli-B"),
        &ControlAction::AnswerPermission { transmission_id: "tx-1".into(), answer: "deny".into() },
    );

    assert!(a.accepted, "first answer is applied");
    assert!(!b.accepted, "second answer is a no-op (already resolved)");
    // The run saw the FIRST decision.
    assert_eq!(rx.await.unwrap(), Decision::Allow);
}
