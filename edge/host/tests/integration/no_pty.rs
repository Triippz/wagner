//! T037b — No interactive shell (FR-302, US3-AS-6, §Out of Scope ③).
//!
//! Negative assertion: the host registers NO PTY/interactive-shell ALPN, and an
//! attempt to open one is REFUSED WITH A PROTOCOL ERROR (not a hang/timeout).
//! Dev-context commands remain non-interactive. This proves the "③ via ssh/tmux,
//! not built into Wagner" boundary is enforced, not merely documented.

use wagner_edge_host::remote::devcontext::{accept_alpn, registered_alpns, AlpnError};

#[test]
fn no_pty_or_shell_alpn_is_registered() {
    for alpn in registered_alpns() {
        assert!(
            !alpn.contains("pty") && !alpn.contains("shell") && !alpn.contains("term"),
            "registered ALPN {alpn} must not be an interactive shell"
        );
    }
}

#[test]
fn opening_an_interactive_shell_alpn_is_refused_with_a_protocol_error() {
    for shell_alpn in ["wagner/pty/1", "wagner/shell/1", "wagner/term/1"] {
        assert_eq!(
            accept_alpn(shell_alpn),
            Err(AlpnError::Unregistered),
            "{shell_alpn} must be refused, not accepted or hung",
        );
    }
}

#[test]
fn the_dev_context_and_control_alpns_are_accepted() {
    assert!(accept_alpn("wagner/devctx/1").is_ok());
    assert!(accept_alpn("wagner/control/1").is_ok());
    assert!(accept_alpn("wagner/attach/1").is_ok());
}
