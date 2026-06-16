//! Tray + window lifecycle (US1). Pure, headless-tested logic; the actual Tauri
//! tray icon, OS notification, and `WindowEvent` handlers (T019) are thin glue
//! over the functions here.
//!
//! Two concerns:
//!  - **Tray status** (FR-102/103): map the run state to a non-color glyph +
//!    label, and raise a notification + badge exactly on entering `needs-you`.
//!  - **Host lifecycle** (FR-101/104): closing the window HIDES it and leaves
//!    the host + log running; only app-quit tears the host down.

use crate::state::RunStatus;

/// The coarse state the tray projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    /// No active run (undrafted or terminal).
    Idle,
    /// A run is executing and needs nothing from the operator.
    Running,
    /// The run is blocked on the operator (an open permission prompt).
    NeedsYou,
}

/// How a tray status is presented — a NON-COLOR glyph + a text label, so the
/// state is legible without relying on color (D-A11Y-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayPresentation {
    pub status: TrayStatus,
    /// Distinct per state; carries the information color alone would not.
    pub glyph: char,
    pub label: &'static str,
    /// A dock/tray badge is shown only when the operator must act.
    pub badge: bool,
}

/// Derive the tray status from the run status + the number of open permission
/// prompts. An open prompt always means `needs-you` (the operator must answer).
pub fn derive_status(run: RunStatus, open_transmissions: usize) -> TrayStatus {
    if open_transmissions > 0 {
        return TrayStatus::NeedsYou;
    }
    match run {
        RunStatus::Running => TrayStatus::Running,
        RunStatus::Drafted
        | RunStatus::Met
        | RunStatus::HaltedGuardrail
        | RunStatus::Aborted
        | RunStatus::Paused => TrayStatus::Idle,
    }
}

/// Present a status as a glyph + label + badge flag.
pub fn present(status: TrayStatus) -> TrayPresentation {
    match status {
        TrayStatus::Idle => TrayPresentation { status, glyph: '○', label: "Idle", badge: false },
        TrayStatus::Running => TrayPresentation { status, glyph: '◔', label: "Running", badge: false },
        TrayStatus::NeedsYou => TrayPresentation { status, glyph: '●', label: "Needs you", badge: true },
    }
}

/// A native notification to raise when the operator's attention is newly needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayNotification {
    pub title: &'static str,
    pub body: &'static str,
}

/// Return a notification ONLY on the edge-transition INTO `needs-you` — never
/// while staying in it (no spam), never on leaving it (SC-008: notify within 5s
/// of *entering* the state; the caller fires this on each status change).
pub fn notification_on_transition(prev: TrayStatus, next: TrayStatus) -> Option<TrayNotification> {
    if next == TrayStatus::NeedsYou && prev != TrayStatus::NeedsYou {
        Some(TrayNotification {
            title: "Wagner needs you",
            body: "A run is waiting on your answer.",
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Host lifecycle
// ---------------------------------------------------------------------------

/// What the window should do in response to a lifecycle event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    /// Hide the window (keep the host alive in the tray).
    Hide,
    /// Exit the app (tear the host down).
    Exit,
}

/// A lifecycle event the Tauri shell forwards to the pure state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// The user closed the window (red button / Cmd-W).
    CloseRequested,
    /// The user reactivated the app (clicked the tray / dock).
    Reopen,
    /// The user quit the app (menu / Cmd-Q).
    AppQuit,
}

/// The host's lifecycle relative to the desktop window. The host process + the
/// append-only event log are deliberately INDEPENDENT of window visibility —
/// closing the window must never stop a run (FR-101, SC-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLifecycle {
    pub window_visible: bool,
    pub host_running: bool,
    pub log_open: bool,
}

impl HostLifecycle {
    pub fn new() -> Self {
        HostLifecycle { window_visible: true, host_running: true, log_open: true }
    }

    /// Apply a lifecycle event, mutating state and returning the window action.
    pub fn on_event(&mut self, event: LifecycleEvent) -> WindowAction {
        match event {
            LifecycleEvent::CloseRequested => {
                // Hide, never exit — the host + log keep running window-closed.
                self.window_visible = false;
                WindowAction::Hide
            }
            LifecycleEvent::Reopen => {
                self.window_visible = true;
                WindowAction::Hide // showing the window; the host was never stopped
            }
            LifecycleEvent::AppQuit => {
                // The one explicit teardown (FR-104).
                self.window_visible = false;
                self.host_running = false;
                self.log_open = false;
                WindowAction::Exit
            }
        }
    }
}

impl Default for HostLifecycle {
    fn default() -> Self {
        Self::new()
    }
}
