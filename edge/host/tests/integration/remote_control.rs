//! T033 + T034 — Remote run-control + gate parity (FR-301/304, US3-AS-1/4/5,
//! SC-002/202).
//!
//! A remote control action is delivered as the SAME control message the local
//! surface sends, routes through the SAME gate seam, advances the run, and is
//! logged ATTRIBUTED to the verified remote operator. A remote action a guardrail
//! would stop locally is stopped identically — origin never weakens the gate.

use wagner_edge_host::orchestrator::guardrails::{check, Verdict};
use wagner_edge_host::remote::control::{route_control, ControlAction, ControlOrigin};
use wagner_edge_host::state::{CostBudget, CostMode, Guardrails, HaltReason};
use wagner_edge_host::transmissions::{Decision, TransmissionRegistry};

fn remote() -> ControlOrigin {
    ControlOrigin::Remote { client_id: "cli-1".into(), operator_id: "op-1".into() }
}

#[tokio::test]
async fn remote_permission_answer_advances_the_run_and_is_attributed() {
    let reg = TransmissionRegistry::default();
    let rx = reg.open("tx-1");
    let outcome = route_control(
        &reg,
        &remote(),
        &ControlAction::AnswerPermission { transmission_id: "tx-1".into(), answer: "allow".into() },
    );
    assert!(outcome.accepted);
    assert_eq!(outcome.gate_decision, Some(Decision::Allow));
    // The audit note attributes the action to the verified remote operator.
    assert!(outcome.note.contains("op-1"));
    assert!(outcome.note.contains("cli-1"));
    // The run advances: the gate resolved the awaited decision.
    assert_eq!(rx.await.unwrap(), Decision::Allow);
}

#[test]
fn a_remote_deny_is_a_deny_identically_to_local() {
    let reg = TransmissionRegistry::default();
    let _rx = reg.open("tx-2");
    let remote_out = route_control(
        &reg,
        &remote(),
        &ControlAction::AnswerPermission { transmission_id: "tx-2".into(), answer: "deny".into() },
    );
    assert_eq!(remote_out.gate_decision, Some(Decision::Deny));

    let reg2 = TransmissionRegistry::default();
    let _rx2 = reg2.open("tx-2");
    let local_out = route_control(
        &reg2,
        &ControlOrigin::Local,
        &ControlAction::AnswerPermission { transmission_id: "tx-2".into(), answer: "deny".into() },
    );
    assert_eq!(local_out.gate_decision, local_out.gate_decision);
    assert_eq!(remote_out.gate_decision, local_out.gate_decision, "origin must not change the decision");
}

#[test]
fn guardrails_are_origin_independent_remote_cannot_bypass() {
    // The guardrail check takes no origin — a remote action faces the identical
    // verdict a local one does for the same run state (SC-202).
    let gr = Guardrails {
        max_iterations: Some(3),
        iterations_used: 3,
        blocked_timeout_secs: 1800,
        cost: CostBudget { mode: CostMode::CliUsage, budget: None, used: 0.0 },
    };
    assert_eq!(check(&gr, 0.0), Verdict::Halt(HaltReason::Iterations));
}
