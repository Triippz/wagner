//! Dev-context ② (T039/T040, FR-302/303, R-6).
//!
//! Two primitives over the attached session, each gated + logged:
//!  - **non-interactive commands** (`wagner/devctx/1`): spawned with piped stdio,
//!    NEVER a PTY (US3-AS-6); output streams to the operator's device as frames
//!    and is not persisted to the log (F-1) — only the invocation + exit are.
//!  - **repo-scoped file reads** (`FR-303`): canonicalised + default-denied
//!    outside the repo root, so out-of-repo secrets are unreachable by
//!    construction (CL-202).
//!
//! No PTY/interactive-shell ALPN is ever registered — an interactive shell is
//! tier ③ (ssh/tmux over the tunnel), explicitly not built into Wagner.

use std::path::{Path, PathBuf};
use std::process::Command;

// --- Repo-scope file guard (FR-303) -------------------------------------------

/// Why a file access was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusedReason {
    /// The path resolves outside the repo root (or does not exist).
    OutOfScope,
}

/// The result of a repo-scope check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAccess {
    /// Allowed — carries the canonicalised in-repo path.
    Allowed(PathBuf),
    Refused(RefusedReason),
}

/// Check a requested path against the repo root. Default-deny: the path is
/// canonicalised (resolving symlinks + `..`) and must lie within the
/// canonicalised root. A non-existent path, a `..` escape, a symlink pointing
/// outside, or an absolute out-of-repo path all refuse.
pub fn check_repo_scope(repo_root: &Path, requested: &Path) -> FileAccess {
    let Ok(canon_root) = repo_root.canonicalize() else {
        return FileAccess::Refused(RefusedReason::OutOfScope);
    };
    let Ok(canon_req) = requested.canonicalize() else {
        // Can't read what doesn't exist / can't resolve — refuse, never panic.
        return FileAccess::Refused(RefusedReason::OutOfScope);
    };
    if canon_req.starts_with(&canon_root) {
        FileAccess::Allowed(canon_req)
    } else {
        FileAccess::Refused(RefusedReason::OutOfScope)
    }
}

// --- ALPN registry (no PTY) ---------------------------------------------------

pub const ALPN_ATTACH: &str = "wagner/attach/1";
pub const ALPN_CONTROL: &str = "wagner/control/1";
pub const ALPN_DEVCTX: &str = "wagner/devctx/1";

/// The ALPNs the host registers — deliberately NO interactive-shell/PTY ALPN
/// (US3-AS-6, §Out of Scope ③).
pub fn registered_alpns() -> &'static [&'static str] {
    &[ALPN_ATTACH, ALPN_CONTROL, ALPN_DEVCTX]
}

/// Why opening a channel ALPN was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpnError {
    /// The ALPN is not one the host serves (e.g. an interactive-shell attempt).
    Unregistered,
}

/// Accept a channel-open for `alpn`, or refuse with a protocol error. An attempt
/// to open an interactive-shell/PTY ALPN is refused here (not left to hang).
pub fn accept_alpn(alpn: &str) -> Result<(), AlpnError> {
    if registered_alpns().contains(&alpn) {
        Ok(())
    } else {
        Err(AlpnError::Unregistered)
    }
}

// --- Non-interactive command (FR-302) -----------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// One framed chunk of command output streamed to the operator's device. This is
/// transient transport — NOT persisted to the log (F-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFrame {
    pub stream: OutputStream,
    pub seq: usize,
    pub chunk: Vec<u8>,
}

/// The log record for a dev-context command — METADATA ONLY: the invocation +
/// its exit, never the output bytes (F-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLog {
    pub argv: Vec<String>,
    pub cwd: String,
    pub exit_code: i32,
}

/// The result of running a non-interactive command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub frames: Vec<OutputFrame>,
    pub log: CommandLog,
}

/// Run a non-interactive command with PIPED stdio (no PTY allocated). stdout is
/// returned as output frames (transient); the log carries argv + exit only.
pub fn run_non_interactive(argv: &[String], cwd: &Path) -> std::io::Result<CommandResult> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty argv"))?;
    // Piped, non-interactive: no PTY, stdin is null.
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .output()?;

    let mut frames = Vec::new();
    if !output.stdout.is_empty() {
        frames.push(OutputFrame { stream: OutputStream::Stdout, seq: 0, chunk: output.stdout.clone() });
    }
    if !output.stderr.is_empty() {
        frames.push(OutputFrame { stream: OutputStream::Stderr, seq: frames.len(), chunk: output.stderr.clone() });
    }
    Ok(CommandResult {
        frames,
        log: CommandLog {
            argv: argv.to_vec(),
            cwd: cwd.display().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        },
    })
}
