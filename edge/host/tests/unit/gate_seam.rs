//! T014a — Gate-reuse seam unit test (FR-004/304, SC-202, R-7).
//!
//! The seam is the SINGLE chokepoint both the local IPC layer and the remote
//! control channel route control actions through. This test proves the gate
//! decision is computed from the ACTION ALONE — a remote-origin action hits the
//! identical decision a local-origin one does, so a remote operator cannot
//! bypass (or weaken, or strengthen) a guardrail the local path enforces. The
//! full channel-wired no-bypass behavior is T034 (Phase 5).

use wagner_edge_host::remote::control::{route_control, ControlAction, ControlOrigin};
use wagner_edge_host::transmissions::{Decision, TransmissionRegistry};

fn local() -> ControlOrigin {
    ControlOrigin::Local
}

fn remote() -> ControlOrigin {
    ControlOrigin::Remote {
        client_id: "cli-1".into(),
        operator_id: "op-1".into(),
    }
}

/// Parametrised: the same permission answer yields the same gate decision and
/// the same transmission resolution regardless of origin.
#[tokio::test]
async fn answer_permission_decision_is_identical_across_origins() {
    for (origin, answer, expected) in [
        (local(), "allow", Decision::Allow),
        (remote(), "allow", Decision::Allow),
        (local(), "deny", Decision::Deny),
        (remote(), "deny", Decision::Deny),
    ] {
        let reg = TransmissionRegistry::default();
        let rx = reg.open("tx-1".into());
        let outcome = route_control(
            &reg,
            &origin,
            &ControlAction::AnswerPermission {
                transmission_id: "tx-1".into(),
                answer: answer.into(),
            },
        );
        assert!(outcome.accepted, "answer must reach the open transmission");
        assert_eq!(
            outcome.gate_decision,
            Some(expected),
            "gate decision must derive from the answer alone, not the origin"
        );
        assert_eq!(rx.await.unwrap(), expected, "the gate resolved the SAME decision");
    }
}

/// A remote answer to a transmission that does not exist is a no-op (it cannot
/// invent a decision the local path wouldn't have).
#[tokio::test]
async fn answer_to_unknown_transmission_is_a_noop_for_any_origin() {
    for origin in [local(), remote()] {
        let reg = TransmissionRegistry::default();
        let outcome = route_control(
            &reg,
            &origin,
            &ControlAction::AnswerPermission {
                transmission_id: "ghost".into(),
                answer: "allow".into(),
            },
        );
        assert!(!outcome.accepted, "no open transmission → not accepted");
        // The decision is still computed identically — origin never changes it.
        assert_eq!(outcome.gate_decision, Some(Decision::Allow));
    }
}

/// Steer and run-skill route through the same seam (no separate remote handler).
#[tokio::test]
async fn steer_and_run_skill_route_through_the_same_seam() {
    let reg = TransmissionRegistry::default();
    for origin in [local(), remote()] {
        let steer = route_control(&reg, &origin, &ControlAction::Steer { text: "go".into() });
        assert!(steer.accepted);
        assert_eq!(steer.gate_decision, None);

        let skill = route_control(
            &reg,
            &origin,
            &ControlAction::RunSkill {
                skill_id: "tdd".into(),
            },
        );
        assert!(skill.accepted);
        assert_eq!(skill.gate_decision, None);
    }
}
