//! Event pipeline: the normalized model + the activity→district routing.

pub mod map_claude;
pub mod map_codex;
pub mod model;
pub mod project;

pub use map_claude::{map_claude_line, tool_to_activity, CliSignal};
pub use map_codex::map_codex_line;
pub use model::{Activity, WagnerEvent, District, Faction, OperativeState};
pub use project::{signal_to_event, EventContext};

/// Map an activity to its district (R-EVENT mapping table from spec.md).
pub fn activity_to_district(activity: Activity) -> District {
    match activity {
        Activity::Read | Activity::Edit => District::Stacks,
        Activity::Test | Activity::Build | Activity::Lint | Activity::Shell => District::Forge,
        Activity::Review | Activity::Diff | Activity::Judge => District::Mirror,
        Activity::Plan | Activity::Decompose | Activity::Think => District::Oracle,
        Activity::AwaitPermission | Activity::AwaitQuestion => District::Gate,
    }
}

/// Default state ring implied by an activity. A blocked operative is always in the Gate.
pub fn activity_to_state(activity: Activity) -> OperativeState {
    match activity {
        Activity::AwaitPermission | Activity::AwaitQuestion => OperativeState::Blocked,
        Activity::Plan | Activity::Decompose | Activity::Think | Activity::Judge => {
            OperativeState::Thinking
        }
        _ => OperativeState::Working,
    }
}
