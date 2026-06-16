//! T025 + T025a — Ephemerality + endpoint-survives-close (FR-101/202, SC-007,
//! US2-AS-6, EC-011).
//!
//! A DELIBERATE close tears down — a re-attach requires re-arm. A TRANSIENT drop
//! while still armed re-attaches without re-arm, but ONLY within the SC-007
//! window: `< 30 s` re-attaches; `= 30 s` and `> 30 s` require re-arm (exclusive
//! upper bound — both sides of the boundary tested). The armed endpoint survives
//! a desktop window-close (the FR-101 endpoint clause, testable now the endpoint
//! exists — challenge C2).

use wagner_edge_host::remote::session::{can_reattach, DropKind, REATTACH_WINDOW_MS};
use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent};

#[test]
fn deliberate_close_requires_rearm() {
    // A deliberate close never permits silent re-attach, regardless of elapsed.
    assert!(!can_reattach(DropKind::Deliberate, 0));
    assert!(!can_reattach(DropKind::Deliberate, 1));
    assert!(!can_reattach(DropKind::Deliberate, REATTACH_WINDOW_MS - 1));
}

#[test]
fn transient_drop_under_30s_reattaches() {
    assert!(can_reattach(DropKind::Transient, 0));
    assert!(can_reattach(DropKind::Transient, 1));
    assert!(can_reattach(DropKind::Transient, REATTACH_WINDOW_MS - 1));
}

#[test]
fn transient_drop_at_exactly_30s_requires_rearm_exclusive_bound() {
    // Exactly the window and beyond → re-arm (the upper bound is exclusive).
    assert!(!can_reattach(DropKind::Transient, REATTACH_WINDOW_MS));
    assert!(!can_reattach(DropKind::Transient, REATTACH_WINDOW_MS + 1));
}

#[test]
fn the_window_is_30_seconds() {
    assert_eq!(REATTACH_WINDOW_MS, 30_000);
}

/// T025a — with the host ARMED, closing the desktop window keeps the host (and
/// thus its iroh endpoint) running, so it keeps accepting attaches. The endpoint
/// is bound to the host lifecycle, not the window.
#[test]
fn armed_endpoint_survives_window_close() {
    let mut lc = HostLifecycle::new();
    // Host is armed (endpoint advertised) when the window closes.
    lc.on_event(LifecycleEvent::CloseRequested);
    assert!(!lc.window_visible, "window hidden");
    assert!(lc.host_running, "host (and its endpoint) keep running window-closed");
    // App-quit is the only thing that tears the endpoint down.
    lc.on_event(LifecycleEvent::AppQuit);
    assert!(!lc.host_running, "app-quit stops the host + endpoint");
}
