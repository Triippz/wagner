//! Async CLI driver — spawns `claude`/`codex` (or a test stand-in) as a child
//! process over pipes (RT-1: stream-json/`exec --json` need no PTY), pumps each
//! stdout line through a mapper into a channel, and allows writing to stdin for
//! permission/question round-trips (US2).

use crate::events::CliSignal;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("failed to spawn `{program}`: {source}")]
    Spawn {
        program: String,
        source: std::io::Error,
    },
    #[error("child stdout/stdin was not captured")]
    NoPipe,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A running CLI subprocess. Drop or `kill` to terminate.
pub struct Driver {
    child: Child,
    stdin: Option<ChildStdin>,
    /// Receives one `CliSignal` per stdout line (already mapped).
    pub signals: mpsc::UnboundedReceiver<CliSignal>,
}

impl Driver {
    /// Spawn `program args...` in `cwd`, mapping each stdout line with `mapper`.
    /// `extra_env` is applied on top of the inherited env; pass nothing that
    /// injects an API key — subscription auth is the CLI's own session (FR-006).
    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: &std::path::Path,
        mapper: fn(&str) -> CliSignal,
    ) -> Result<Self, DriverError> {
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| DriverError::Spawn {
                program: program.to_string(),
                source,
            })?;

        let stdout = child.stdout.take().ok_or(DriverError::NoPipe)?;
        let stdin = child.stdin.take();
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        // A closed receiver means the run was aborted; stop pumping.
                        if tx.send(mapper(&line)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break, // clean EOF — child closed stdout
                    Err(e) => {
                        // A read error must not silently stall the run with no
                        // diagnosis path (M7); log it before stopping the pump.
                        eprintln!("[wagner] driver stdout read error: {e}");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            signals: rx,
        })
    }

    /// Write a line to the child's stdin (e.g. a permission decision). Appends
    /// a newline. Used to answer transmissions (US2).
    pub async fn write_line(&mut self, line: &str) -> Result<(), DriverError> {
        let stdin = self.stdin.as_mut().ok_or(DriverError::NoPipe)?;
        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Wait for the child to exit and return whether it succeeded.
    pub async fn wait(&mut self) -> Result<bool, DriverError> {
        let status = self.child.wait().await?;
        Ok(status.success())
    }

    /// Forcibly terminate the child (abort path, FR-017).
    pub async fn kill(&mut self) -> Result<(), DriverError> {
        Ok(self.child.kill().await?)
    }

    /// Drain all remaining signals after the child has finished emitting.
    pub async fn collect_remaining(&mut self) -> Vec<CliSignal> {
        let mut out = Vec::new();
        while let Some(sig) = self.signals.recv().await {
            out.push(sig);
        }
        out
    }
}
