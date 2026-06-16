//! Project a normalized `CliSignal` into a `WagnerEvent` for the frontend.
//!
//! This is the seam between "what the CLI did" and "what the operative does on
//! the floor". Pure + testable; the loop supplies the run/operative identity and
//! the (non-deterministic) event id + timestamp.

use super::{activity_to_district, Activity, CliSignal, WagnerEvent, Faction, OperativeState};

/// Identity + clock the loop injects when projecting a signal.
pub struct EventContext<'a> {
    pub run_id: &'a str,
    pub operative_id: &'a str,
    /// Display name of the hired agent (floor label).
    pub operative_name: &'a str,
    pub faction: Faction,
    pub event_id: String,
    pub ts: String,
}

/// Turn a signal into a WagnerEvent, or `None` for signals with no visual
/// meaning (`Ignored`).
pub fn signal_to_event(signal: &CliSignal, ctx: EventContext) -> Option<WagnerEvent> {
    let (activity, state, message) = match signal {
        // Operative just appeared — it's in the Oracle, idle, about to think.
        CliSignal::Spawned => (Activity::Think, OperativeState::Idle, None),
        CliSignal::Activity { activity, message } => (
            *activity,
            super::activity_to_state(*activity),
            message.clone(),
        ),
        CliSignal::AwaitingInput { prompt } => (
            Activity::AwaitPermission,
            OperativeState::Blocked,
            Some(prompt.clone()),
        ),
        // Finished — step into the Mirror, idle.
        CliSignal::Completed { result, .. } => (
            Activity::Review,
            OperativeState::Idle,
            (!result.is_empty()).then(|| result.clone()),
        ),
        CliSignal::Ignored => return None,
    };

    Some(WagnerEvent {
        schema: WagnerEvent::SCHEMA.to_string(),
        event_id: ctx.event_id,
        run_id: ctx.run_id.to_string(),
        operative_id: ctx.operative_id.to_string(),
        operative_name: ctx.operative_name.to_string(),
        faction: ctx.faction,
        activity,
        district: activity_to_district(activity),
        state,
        message,
        handoff_target_operative_id: None,
        ts: ctx.ts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::District;

    fn ctx() -> EventContext<'static> {
        EventContext {
            run_id: "r1",
            operative_id: "cipher",
            operative_name: "Cipher",
            faction: Faction::Architects,
            event_id: "01J0000000000000000000000E".to_string(),
            ts: "2026-06-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn edit_activity_projects_to_stacks_working() {
        let s = CliSignal::Activity {
            activity: Activity::Edit,
            message: Some("editing x".into()),
        };
        let e = signal_to_event(&s, ctx()).unwrap();
        assert_eq!(e.district, District::Stacks);
        assert_eq!(e.state, OperativeState::Working);
        assert_eq!(e.message.as_deref(), Some("editing x"));
    }

    #[test]
    fn awaiting_input_projects_to_gate_blocked() {
        let s = CliSignal::AwaitingInput {
            prompt: "allow write?".into(),
        };
        let e = signal_to_event(&s, ctx()).unwrap();
        assert_eq!(e.district, District::Gate);
        assert_eq!(e.state, OperativeState::Blocked);
    }

    #[test]
    fn spawned_projects_to_oracle_idle() {
        let e = signal_to_event(&CliSignal::Spawned, ctx()).unwrap();
        assert_eq!(e.district, District::Oracle);
        assert_eq!(e.state, OperativeState::Idle);
    }

    #[test]
    fn completed_projects_to_mirror() {
        let s = CliSignal::Completed {
            cost_usd: Some(0.1),
            tokens: None,
            result: "done".into(),
        };
        let e = signal_to_event(&s, ctx()).unwrap();
        assert_eq!(e.district, District::Mirror);
        assert_eq!(e.message.as_deref(), Some("done"));
    }

    #[test]
    fn ignored_yields_no_event() {
        assert!(signal_to_event(&CliSignal::Ignored, ctx()).is_none());
    }

    #[test]
    fn event_validates_against_schema() {
        let e = signal_to_event(&CliSignal::Spawned, ctx()).unwrap();
        crate::schema::validate_serialized(crate::schema::CONSTRUCT_EVENT_SCHEMA, &e)
            .expect("projected event must validate");
    }
}
