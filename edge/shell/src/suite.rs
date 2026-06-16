//! The goal-met suite gate (FR-013b).
//!
//! Design note (autonomous-mode resolution): The Construct builds *arbitrary*
//! projects, so it cannot know how to run an unknown project's full test suite
//! generically. v1 resolution — the Claude judge pass is instructed to run the
//! project's tests as part of its confirmation, and this host-level gate trusts
//! that unless an explicit suite command is configured for the run. A configured
//! command is shelled out and its exit status is the suite verdict.

use wagner_edge_host::orchestrator::judge::SuiteResult;
use std::path::Path;

/// Run the configured suite command (if any) in the run's project directory and
/// return its verdict. With no command, returns `passed: true` (the judge carries
/// the suite check). Synchronous + blocking; the loop calls it at most once per
/// iteration. `cwd` is the selected project directory so the suite runs against
/// the target repo, not The Construct's own tree.
pub fn run_suite(command: Option<&str>, cwd: &Path) -> SuiteResult {
    let Some(cmd) = command else {
        return SuiteResult { passed: true };
    };
    // Parse the configured command into an argv vector and spawn the program
    // DIRECTLY — never via `sh -c` — so a frontend-supplied string cannot inject
    // extra shell commands (M1, shell-injection). A command that fails to parse
    // (unbalanced quotes) or is empty fails closed.
    let Ok(argv) = shell_words::split(cmd) else {
        return SuiteResult { passed: false };
    };
    let Some((program, args)) = argv.split_first() else {
        return SuiteResult { passed: false };
    };
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status();
    SuiteResult {
        passed: matches!(status, Ok(s) if s.success()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_command_trusts_the_judge() {
        assert!(run_suite(None, Path::new(".")).passed);
    }

    #[test]
    fn passing_command_is_green() {
        assert!(run_suite(Some("true"), Path::new(".")).passed);
    }

    #[test]
    fn failing_command_is_red() {
        assert!(!run_suite(Some("false"), Path::new(".")).passed);
    }

    /// The suite command runs in the selected project directory, not in The
    /// Construct's own cwd — proven by testing for a marker file that exists only
    /// in a temp dir we pass as `cwd`.
    #[test]
    fn command_runs_in_the_given_project_dir() {
        let dir = std::env::temp_dir().join(format!("wagner-suite-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("MARKER"), "x").unwrap();
        assert!(run_suite(Some("test -f MARKER"), &dir).passed);
        // The same check fails from a dir without the marker.
        assert!(!run_suite(Some("test -f MARKER"), Path::new("/")).passed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
