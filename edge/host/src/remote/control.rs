//! Gate-reuse seam (T014, R-7, FR-004/304).
//!
//! THE single internal entrypoint for run-control ① actions. Both the carried
//! local IPC layer and the remote control channel (wired in Phase 5, T038) call
//! `route_control`; there is no second gate. The action's ORIGIN is recorded for
//! audit (the remote variant carries the verified operator + client) but is
//! NEVER consulted when computing the gate decision — a remote action is gated
//! identically to a local one, so it cannot bypass a guardrail (SC-202) and the
//! audit trail is complete (SC-002).

use crate::transmissions::{Decision, TransmissionRegistry};

/// Where a control action originated. Audited, never used for the decision.
#[derive(Debug, Clone)]
pub enum ControlOrigin {
    /// The local desktop IPC layer (carried Tauri commands).
    Local,
    /// A verified remote operator over the attached control channel.
    Remote { client_id: String, operator_id: String },
}

impl ControlOrigin {
    /// Audit label appended to the `remote.control` event metadata.
    pub fn label(&self) -> String {
        match self {
            ControlOrigin::Local => "local".to_string(),
            ControlOrigin::Remote { client_id, operator_id } => {
                format!("remote:{operator_id}/{client_id}")
            }
        }
    }
}

/// A run-control ① intent: steer the run, answer an open permission, or launch
/// a skill. Identical whether it came from local IPC or a remote channel.
#[derive(Debug, Clone)]
pub enum ControlAction {
    AnswerPermission { transmission_id: String, answer: String },
    Steer { text: String },
    RunSkill { skill_id: String },
}

/// What the seam decided + did. `gate_decision` is `Some` only for permission
/// answers (the gated path); steer/run-skill carry `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlOutcome {
    /// Whether the action was applied (e.g. the transmission was open).
    pub accepted: bool,
    /// The gate decision, when this action passed through the permission gate.
    pub gate_decision: Option<Decision>,
    /// Human-readable note for logs.
    pub note: String,
}

/// Route a control action through the carried gate. Single chokepoint (R-7).
///
/// For `AnswerPermission`, the decision is derived from the answer ALONE via the
/// carried `Decision::from_answer` and delivered through the carried
/// `TransmissionRegistry` — exactly the path the local IPC `answer_transmission`
/// uses. `origin` does not enter the decision, which is precisely what makes a
/// remote action unable to bypass the gate (FR-304, SC-202).
pub fn route_control(
    reg: &TransmissionRegistry,
    origin: &ControlOrigin,
    action: &ControlAction,
) -> ControlOutcome {
    match action {
        ControlAction::AnswerPermission { transmission_id, answer } => {
            // Origin-independent: the same answer always maps to the same decision.
            let decision = Decision::from_answer(answer);
            let delivered = reg.answer(transmission_id, decision);
            ControlOutcome {
                accepted: delivered,
                gate_decision: Some(decision),
                note: if delivered {
                    format!("{} answered {transmission_id} ({:?})", origin.label(), decision)
                } else {
                    format!("{} answered {transmission_id} — no open transmission", origin.label())
                },
            }
        }
        ControlAction::Steer { text } => ControlOutcome {
            accepted: true,
            gate_decision: None,
            note: format!("{} steer: {} chars", origin.label(), text.len()),
        },
        ControlAction::RunSkill { skill_id } => ControlOutcome {
            accepted: true,
            gate_decision: None,
            note: format!("{} run_skill: {skill_id}", origin.label()),
        },
    }
}
