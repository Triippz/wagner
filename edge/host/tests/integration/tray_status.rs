//! T017 — Tray status + needs-you (FR-102/103, US1-AS-3/4, SC-008, D-A11Y-1).
//!
//! Headless-tested pure logic: the tray maps the run state to a NON-COLOR
//! glyph and text label, and a transition INTO `needs-you` (an open permission
//! prompt) raises a native notification + badge exactly once (not on every
//! poll). The actual Tauri tray icon / OS notification call is thin glue over
//! these pure functions (T019); the behavior under test is the mapping + the
//! edge-trigger.

use wagner_edge_host::state::RunStatus;
use wagner_edge_host::tray::{derive_status, notification_on_transition, present, TrayStatus};

#[test]
fn running_with_no_prompt_is_running() {
    assert_eq!(derive_status(RunStatus::Running, 0), TrayStatus::Running);
}

#[test]
fn an_open_permission_prompt_makes_it_needs_you() {
    assert_eq!(derive_status(RunStatus::Running, 1), TrayStatus::NeedsYou);
}

#[test]
fn terminal_or_undrafted_states_are_idle() {
    for s in [RunStatus::Drafted, RunStatus::Met, RunStatus::HaltedGuardrail, RunStatus::Aborted] {
        assert_eq!(derive_status(s, 0), TrayStatus::Idle, "{s:?} should be idle");
    }
}

#[test]
fn each_status_has_a_distinct_non_color_glyph_and_label() {
    let idle = present(TrayStatus::Idle);
    let running = present(TrayStatus::Running);
    let needs = present(TrayStatus::NeedsYou);
    // Distinct glyphs (information not carried by color alone — D-A11Y-1).
    let glyphs = [idle.glyph, running.glyph, needs.glyph];
    assert_eq!(glyphs.iter().collect::<std::collections::HashSet<_>>().len(), 3);
    // Non-empty text labels on every state.
    for p in [&idle, &running, &needs] {
        assert!(!p.label.is_empty());
    }
    // Only needs-you carries a badge.
    assert!(!idle.badge && !running.badge && needs.badge);
}

#[test]
fn notification_raised_only_on_the_transition_into_needs_you() {
    // Entering needs-you raises one notification.
    assert!(notification_on_transition(TrayStatus::Running, TrayStatus::NeedsYou).is_some());
    assert!(notification_on_transition(TrayStatus::Idle, TrayStatus::NeedsYou).is_some());
    // Staying in needs-you does NOT re-raise (no notification spam).
    assert!(notification_on_transition(TrayStatus::NeedsYou, TrayStatus::NeedsYou).is_none());
    // Leaving needs-you raises nothing.
    assert!(notification_on_transition(TrayStatus::NeedsYou, TrayStatus::Running).is_none());
    assert!(notification_on_transition(TrayStatus::Running, TrayStatus::Idle).is_none());
}
