//! Voice projection participant (spec 015, US4) — speaks an **allowlist** of bus
//! events aloud via the existing `Tts` port, and stays silent on everything else
//! (FR-009). The audio-playing `Agent` impl (subscribe → `Tts` → cpal playback)
//! lands with the device layer; the **speak policy** below is pure, deterministic,
//! and headless-testable so the allowlist is verified without any audio.
//!
//! Allowlist (FR-009), mapped onto the current `RunEvent` taxonomy:
//! - `Finished { ok: true }`  → "Run complete."
//! - `Finished { ok: false }` → "Run stopped."        (stopped / aborted)
//! - `Transmission`           → "Waiting for your approval."  (gate-blocked / awaiting answer)
//!
//! Everything else is silent. The set is **additive-versioned**: when an explicit
//! assistant-conversation event lands, it joins the match below — no rework.

use crate::bus::{Event, RunEvent};

/// The projection allowlist (FR-009): the spoken text for a speakable event, or
/// `None` to stay silent. This is the single source of truth for what voice says.
pub fn speakable_text(event: &Event) -> Option<String> {
    match event {
        Event::Run(RunEvent::Finished { ok: true, .. }) => Some("Run complete.".to_string()),
        Event::Run(RunEvent::Finished { ok: false, .. }) => Some("Run stopped.".to_string()),
        Event::Run(RunEvent::Transmission(_)) => Some("Waiting for your approval.".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{GoalEvent, UiEvent, VaultEvent, VoiceEvent};
    use serde_json::json;

    fn finished(ok: bool) -> Event {
        Event::Run(RunEvent::Finished { run_id: "r1".into(), ok })
    }

    #[test]
    fn run_complete_speaks() {
        assert_eq!(speakable_text(&finished(true)).as_deref(), Some("Run complete."));
    }

    #[test]
    fn run_stopped_speaks() {
        assert_eq!(speakable_text(&finished(false)).as_deref(), Some("Run stopped."));
    }

    #[test]
    fn awaiting_approval_speaks() {
        let ev = Event::Run(RunEvent::Transmission(json!({ "tool": "Bash" })));
        assert_eq!(speakable_text(&ev).as_deref(), Some("Waiting for your approval."));
    }

    #[test]
    fn non_allowlisted_events_are_silent() {
        // A representative spread of the firehose — none of these is spoken (SC-004).
        let silent = [
            Event::Goal(GoalEvent::Added { goal_id: "g1".into(), title: "x".into() }),
            Event::Vault(VaultEvent::NoteUpdated { path: "n.md".into(), rev: 1 }),
            Event::Ui(UiEvent::SurfaceFocused { surface: "console".into() }),
            Event::Voice(VoiceEvent::UtteranceTranscribed { text: "hello".into() }),
            Event::Run(RunEvent::WorkflowStep(json!({}))),
            Event::Run(RunEvent::WorkflowDone(json!({}))),
        ];
        for ev in silent {
            assert_eq!(speakable_text(&ev), None, "{ev:?} must be silent");
        }
    }
}
