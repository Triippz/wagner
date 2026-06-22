//! Command → run routing — the engine-side bridge that turns a dispatched
//! `RunCommand` into a launched/steered/aborted run. Replaces the 011-P3 log-only
//! command-intake drain so **any** transport that dispatches through the bus — the
//! desktop UI, a voice participant, a future remote/browser client — actually acts.
//!
//! ADR-0004: the **engine** owns the routing and the [`RunLaunch`] *port*; the
//! platform-specific launch (CLI agent pool, permission gate, suite runner) is the
//! *adapter* the shell (or a future headless daemon) injects. This keeps run-launch
//! reachable from every client/transport without binding it to the Tauri shell.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::bus::{Command, CommandEnvelope, RunCommand};

/// The platform port that actually launches/steers/aborts a run. The shell provides
/// the adapter (its CLI pool + gate + suite); the engine drives it from the bus.
#[async_trait]
pub trait RunLaunch: Send + Sync {
    /// Launch a new run for `goal` (against the launcher's default workspace);
    /// returns the new run id.
    async fn launch(&self, goal: String) -> Result<String, String>;
    /// Inject a live steering instruction into `run_id`.
    async fn steer(&self, run_id: String, text: String) -> Result<(), String>;
    /// Abort a run (`None` = the launcher's single live session).
    async fn abort(&self, run_id: Option<String>) -> Result<(), String>;
}

/// Drains the bus command-intake and routes each `RunCommand` to the [`RunLaunch`]
/// port. The single consumer of `Bus::take_commands()`.
pub struct RunCommandRouter {
    launcher: Arc<dyn RunLaunch>,
}

impl RunCommandRouter {
    pub fn new(launcher: Arc<dyn RunLaunch>) -> Self {
        Self { launcher }
    }

    /// Route commands until the intake channel closes (app exit).
    pub async fn run(&self, mut commands: mpsc::Receiver<CommandEnvelope>) {
        while let Some(env) = commands.recv().await {
            self.route(env.command).await;
        }
    }

    /// Route one command. A launch error is logged, never fatal — one bad command
    /// must not stop the router (the dispatcher already validated + authorized it).
    /// Non-run commands are ignored here (other namespaces route elsewhere).
    async fn route(&self, command: Command) {
        let result = match command {
            Command::Run(RunCommand::Start { goal }) => self.launcher.launch(goal).await.map(|_| ()),
            Command::Run(RunCommand::Steer { run_id, text }) => self.launcher.steer(run_id, text).await,
            Command::Run(RunCommand::Abort { run_id }) => self.launcher.abort(run_id).await,
            other => {
                // Goal/Vault/Voice/Ui/Ext are not this router's concern.
                eprintln!("[wagner] run-router: ignoring non-run command {other:?}");
                Ok(())
            }
        };
        if let Err(e) = result {
            eprintln!("[wagner] run-router: command failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{EventId, GoalCommand};
    use std::sync::Mutex;

    /// Records every call so the routing can be asserted; `fail_launch` forces the
    /// next launch to error (to prove one failure doesn't stop the router).
    #[derive(Default)]
    struct FakeLaunch {
        launched: Mutex<Vec<String>>,
        steered: Mutex<Vec<(String, String)>>,
        aborted: Mutex<Vec<Option<String>>>,
        fail_launch: bool,
    }

    #[async_trait]
    impl RunLaunch for FakeLaunch {
        async fn launch(&self, goal: String) -> Result<String, String> {
            self.launched.lock().unwrap().push(goal);
            if self.fail_launch {
                Err("forced failure".into())
            } else {
                Ok("run-1".into())
            }
        }
        async fn steer(&self, run_id: String, text: String) -> Result<(), String> {
            self.steered.lock().unwrap().push((run_id, text));
            Ok(())
        }
        async fn abort(&self, run_id: Option<String>) -> Result<(), String> {
            self.aborted.lock().unwrap().push(run_id);
            Ok(())
        }
    }

    fn env(command: Command) -> CommandEnvelope {
        CommandEnvelope {
            id: EventId("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()),
            command,
        }
    }

    /// Drive the router over a fixed list of commands (channel closes → `run` returns).
    async fn route_all(launcher: Arc<FakeLaunch>, commands: Vec<Command>) {
        let (tx, rx) = mpsc::channel(16);
        for c in commands {
            tx.send(env(c)).await.unwrap();
        }
        drop(tx); // close so run() terminates
        RunCommandRouter::new(launcher).run(rx).await;
    }

    #[tokio::test]
    async fn routes_start_to_launch() {
        let fake = Arc::new(FakeLaunch::default());
        route_all(
            Arc::clone(&fake),
            vec![Command::Run(RunCommand::Start { goal: "research X".into() })],
        )
        .await;
        assert_eq!(fake.launched.lock().unwrap().as_slice(), ["research X"]);
    }

    #[tokio::test]
    async fn routes_steer_and_abort() {
        let fake = Arc::new(FakeLaunch::default());
        route_all(
            Arc::clone(&fake),
            vec![
                Command::Run(RunCommand::Steer { run_id: "r7".into(), text: "use the other approach".into() }),
                Command::Run(RunCommand::Abort { run_id: None }),
            ],
        )
        .await;
        assert_eq!(fake.steered.lock().unwrap().as_slice(), [("r7".to_string(), "use the other approach".to_string())]);
        assert_eq!(fake.aborted.lock().unwrap().as_slice(), [None]);
    }

    #[tokio::test]
    async fn ignores_non_run_commands() {
        let fake = Arc::new(FakeLaunch::default());
        route_all(
            Arc::clone(&fake),
            vec![Command::Goal(GoalCommand::Add { title: "later".into() })],
        )
        .await;
        assert!(fake.launched.lock().unwrap().is_empty(), "a non-run command must not launch");
    }

    #[tokio::test]
    async fn a_launch_failure_does_not_stop_the_router() {
        let fake = Arc::new(FakeLaunch { fail_launch: true, ..Default::default() });
        route_all(
            Arc::clone(&fake),
            vec![
                Command::Run(RunCommand::Start { goal: "first (fails)".into() }),
                Command::Run(RunCommand::Start { goal: "second".into() }),
            ],
        )
        .await;
        // Both were routed even though the first errored.
        assert_eq!(fake.launched.lock().unwrap().len(), 2, "router survives a launch error");
    }
}
