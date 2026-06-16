//! T015 — Host lifecycle (FR-101 host+log clause, FR-104, US1-AS-1/5, EC-007).
//!
//! Headless-tested pure logic: closing the desktop WINDOW must HIDE it (not
//! exit), leaving the host process + its append-only event log running; only an
//! explicit app-quit tears the host down. The real `WindowEvent::CloseRequested`
//! handler (T019) is thin glue calling `HostLifecycle::on_event`; the behavior
//! under test is the decision + the survival of the host across a window close.
//! (The FR-101 remote-endpoint-survival clause is tested in US2/T025a, once the
//! endpoint exists — challenge C2.)

use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent, WindowAction};

#[test]
fn close_requested_hides_the_window_but_keeps_the_host_and_log_running() {
    let mut lc = HostLifecycle::new();
    assert!(lc.window_visible && lc.host_running && lc.log_open);

    let action = lc.on_event(LifecycleEvent::CloseRequested);

    assert_eq!(action, WindowAction::Hide, "close must hide, never exit");
    assert!(!lc.window_visible, "window is hidden");
    assert!(lc.host_running, "the host process keeps running window-closed");
    assert!(lc.log_open, "the append-only event log survives window-close");
}

#[test]
fn reopening_after_close_shows_the_window_again_without_restarting_the_host() {
    let mut lc = HostLifecycle::new();
    lc.on_event(LifecycleEvent::CloseRequested);
    lc.on_event(LifecycleEvent::Reopen);
    assert!(lc.window_visible);
    assert!(lc.host_running, "reopening must not have restarted the host");
}

#[test]
fn app_quit_stops_the_host_and_closes_the_log() {
    let mut lc = HostLifecycle::new();
    let action = lc.on_event(LifecycleEvent::AppQuit);
    assert_eq!(action, WindowAction::Exit);
    assert!(!lc.host_running, "app-quit is the explicit host teardown (FR-104)");
    assert!(!lc.log_open);
}
