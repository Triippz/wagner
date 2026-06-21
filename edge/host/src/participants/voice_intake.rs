//! Voice intake participant (spec 015, US1/US2/US3-spoken) — turns a transcript
//! into an action on the bus. The capture → AEC → STT wiring and the `Agent` impl
//! land with the device layer; the **routing decision** below is pure,
//! deterministic, and headless-testable (the shared path both PTT and wake feed).
//!
//! Routing (FR-005a / FR-006 / FR-007):
//! 1. A best-effort **spoken cancel** ("stop", "never mind") → [`IntakeAction::Cancel`]
//!    (the participant delivers this to `registry.cancel` directly — the council's
//!    deterministic abort lives on the *physical* control; this spoken path is the
//!    flexible convenience).
//! 2. Otherwise it is **free-form**: a **focused run** → [`IntakeAction::Steer`];
//!    no focused run → [`IntakeAction::StartRun`].

use crate::bus::{Command, RunCommand};
use crate::voice::{classify_spoken, SpokenIntent};

/// What a transcript routes to. The participant maps `Cancel` to `registry.cancel`
/// (bypassing dispatch, per the 014 abort path); `StartRun`/`Steer` map to a
/// validated [`Command`] via [`IntakeAction::to_command`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntakeAction {
    /// Best-effort spoken cancel → run-cancel action.
    Cancel,
    /// No focused run → start a new run with this goal.
    StartRun { goal: String },
    /// A run is focused → steer it with this text.
    Steer { run_id: String, text: String },
}

/// Route a transcript given the currently focused run, if any (FR-006/FR-007).
///
/// `focused_run` is a snapshot taken at dispatch time. EC-007 (the focused run
/// terminates while the utterance was being transcribed) is handled by the caller
/// re-checking liveness and passing `None` — a terminal run is never steered.
pub fn route_transcript(transcript: &str, focused_run: Option<&str>) -> IntakeAction {
    if classify_spoken(transcript) == SpokenIntent::Cancel {
        return IntakeAction::Cancel;
    }
    let text = transcript.trim().to_string();
    match focused_run {
        Some(run_id) => IntakeAction::Steer { run_id: run_id.to_string(), text },
        None => IntakeAction::StartRun { goal: text },
    }
}

impl IntakeAction {
    /// The validated [`Command`] for the free-form actions. `Cancel` returns `None`
    /// because the spoken cancel is delivered to `registry.cancel` directly, not
    /// through the command intake (014 abort path).
    pub fn to_command(&self) -> Option<Command> {
        match self {
            IntakeAction::Cancel => None,
            IntakeAction::StartRun { goal } => Some(Command::Run(RunCommand::Start { goal: goal.clone() })),
            IntakeAction::Steer { run_id, text } => {
                Some(Command::Run(RunCommand::Steer { run_id: run_id.clone(), text: text.clone() }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_cancel_routes_to_cancel_regardless_of_focus() {
        assert_eq!(route_transcript("stop", Some("r1")), IntakeAction::Cancel);
        assert_eq!(route_transcript("never mind", None), IntakeAction::Cancel);
    }

    #[test]
    fn free_form_with_no_focus_starts_a_run() {
        assert_eq!(
            route_transcript("research the voice landscape", None),
            IntakeAction::StartRun { goal: "research the voice landscape".into() }
        );
    }

    #[test]
    fn free_form_with_focus_steers_that_run() {
        assert_eq!(
            route_transcript("use the other approach", Some("run-7")),
            IntakeAction::Steer { run_id: "run-7".into(), text: "use the other approach".into() }
        );
    }

    #[test]
    fn cancel_word_with_trailing_goal_is_not_a_cancel() {
        // "stop wasting tokens" is a steer/goal, never an abort (council safety case).
        assert_eq!(
            route_transcript("stop wasting tokens", Some("r1")),
            IntakeAction::Steer { run_id: "r1".into(), text: "stop wasting tokens".into() }
        );
        assert_eq!(
            route_transcript("stop wasting tokens", None),
            IntakeAction::StartRun { goal: "stop wasting tokens".into() }
        );
    }

    #[test]
    fn to_command_maps_free_form_and_skips_cancel() {
        assert_eq!(IntakeAction::Cancel.to_command(), None);
        assert_eq!(
            IntakeAction::StartRun { goal: "g".into() }.to_command(),
            Some(Command::Run(RunCommand::Start { goal: "g".into() }))
        );
        assert_eq!(
            IntakeAction::Steer { run_id: "r".into(), text: "t".into() }.to_command(),
            Some(Command::Run(RunCommand::Steer { run_id: "r".into(), text: "t".into() }))
        );
    }
}
