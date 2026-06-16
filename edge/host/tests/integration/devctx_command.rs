//! T035 — Non-interactive dev-context command (FR-302, US3-AS-2).
//!
//! A command streams stdout/stderr back as piped frames (no PTY); the command +
//! exit are logged, but the OUTPUT is not persisted to the log (F-1).

use wagner_edge_host::remote::devcontext::{run_non_interactive, CommandLog, OutputStream};

#[test]
fn command_streams_piped_output_and_logs_metadata_only() {
    let argv: Vec<String> = ["echo", "hello-remote"].iter().map(|s| s.to_string()).collect();
    let result = run_non_interactive(&argv, std::path::Path::new(".")).expect("echo runs");

    // stdout came back as a frame (transient transport to the operator device).
    let stdout = result
        .frames
        .iter()
        .find(|f| f.stream == OutputStream::Stdout)
        .expect("stdout frame");
    assert!(String::from_utf8_lossy(&stdout.chunk).contains("hello-remote"));

    // The LOG carries argv + exit only — structurally there is NO output field
    // for captured stdout/stderr to leak into (F-1). The output lives in the
    // transient frames above, never in the log record.
    assert_eq!(result.log.argv, argv);
    assert_eq!(result.log.exit_code, 0);
    let CommandLog { argv: _, cwd: _, exit_code: _ } = &result.log; // exhaustive: no output field
}

#[test]
fn a_failing_command_reports_its_nonzero_exit() {
    let argv: Vec<String> = ["false"].iter().map(|s| s.to_string()).collect();
    let result = run_non_interactive(&argv, std::path::Path::new(".")).expect("false runs");
    assert_ne!(result.log.exit_code, 0);
}
