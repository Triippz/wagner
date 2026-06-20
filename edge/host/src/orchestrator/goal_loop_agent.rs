//! The goal loop as a bus participant (spec 011 P4).
//!
//! Today the goal loop sits at the centre and the shell drives it imperatively.
//! [`GoalLoopAgent`] inverts that: it wraps [`run_goal`] and translates its
//! imperative `emit`/`progress`/`emit_panel` callbacks into facts **published on
//! the bus** via an [`AgentContext`] — so the loop is one participant among many
//! (connectors, the scheduler, the UI) rather than the hub. The execution deps
//! the shell assembles (the agent `pool`, the runs root, the test suite) are
//! still injected; only the observability side moves onto the bus.

use std::path::Path;

use futures::future::BoxFuture;

use crate::bus::{AgentContext, Event, RunEvent, StreamId, UiEvent};
use crate::orchestrator::judge::SuiteResult;
use crate::orchestrator::run_loop::{run_goal, AgentPool, LoopDeps};
use crate::state::{ConsoleInput, HaltReason, Run};

/// The goal loop wrapped as a bus participant. Holds the [`AgentContext`] it
/// publishes facts through; one instance drives one run to completion.
pub struct GoalLoopAgent {
    ctx: AgentContext,
    /// 014 US1: cooperative cancel signal threaded into the loop (FR-013). `None`
    /// for the non-cancellable path (the existing goal-loop-as-agent tests).
    cancel: Option<tokio::sync::watch::Receiver<bool>>,
}

impl GoalLoopAgent {
    pub fn new(ctx: AgentContext) -> Self {
        Self { ctx, cancel: None }
    }

    /// Thread a cooperative cancel signal into the loop (registry-supervised runs).
    pub fn with_cancel(mut self, cancel: tokio::sync::watch::Receiver<bool>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Drive `run` to completion with `pool`, publishing every loop signal as a
    /// fact on the run's stream — operative `Activity` (`wagner://event`), live
    /// `Snapshot`s (`wagner://run`), and agent `Panel`s (`wagner://panel`) — plus
    /// a final terminal `Snapshot`. Returns the final [`Run`].
    ///
    /// Steering + external-halt are no-ops here: in the participant model those
    /// arrive as commands (`run.steer` / `run.abort`) routed by the registry, not
    /// as injected closures.
    pub async fn run(
        &self,
        run: Run,
        pool: &dyn AgentPool,
        runs_root: &Path,
        run_suite: &(dyn Fn() -> BoxFuture<'static, SuiteResult> + Send + Sync),
    ) -> Run {
        let ctx = &self.ctx;
        let run_id = run.run_id.clone();

        let emit = |ev: crate::events::WagnerEvent| {
            ctx.publish(StreamId::Run(run_id.clone()), Event::Run(RunEvent::Activity(Box::new(ev))));
        };
        let progress = |r: &Run| {
            ctx.publish(StreamId::Run(run_id.clone()), Event::Run(RunEvent::Snapshot(Box::new(r.clone()))));
        };
        let emit_panel = |operative_id: &str, spec: serde_json::Value| {
            ctx.publish(
                StreamId::Run(run_id.clone()),
                Event::Ui(UiEvent::Panel { operative_id: operative_id.to_string(), spec }),
            );
        };
        let no_steer = || Vec::<ConsoleInput>::new();
        let no_halt = || None::<HaltReason>;

        let final_run = run_goal(
            run,
            LoopDeps {
                pool,
                run_suite,
                runs_root,
                emit: &emit,
                steer: &no_steer,
                external_halt: &no_halt,
                progress: &progress,
                emit_panel: &emit_panel,
                cancel: self.cancel.clone(),
            },
        )
        .await;

        // Terminal fact: the loop's last word, on the bus like everything else.
        ctx.publish(
            StreamId::Run(run_id.clone()),
            Event::Run(RunEvent::Snapshot(Box::new(final_run.clone()))),
        );
        final_run
    }
}
