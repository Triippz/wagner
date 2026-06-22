//! Voice intake participant (spec 015, US1/US2/US3-spoken) — turns a transcript
//! into an action on the bus. The capture → AEC → STT wiring is device-gated and
//! publishes `voice.utterance_transcribed`; the `Agent` impl (subscribe + dispatch)
//! and the **routing decision** below are both headless-testable
//! (the shared path both PTT and wake feed).
//!
//! Routing (FR-005a / FR-006 / FR-007):
//! 1. A best-effort **spoken cancel** ("stop", "never mind") → [`IntakeAction::Cancel`]
//!    (the participant dispatches `run.abort` through the bounded command intake;
//!    failure is swallowed on backpressure — the *physical* control carries the
//!    deterministic abort guarantee; this spoken path is the flexible convenience).
//! 2. Otherwise it is **free-form**: a **focused run** → [`IntakeAction::Steer`];
//!    no focused run → [`IntakeAction::StartRun`].

use std::sync::Arc;

use async_trait::async_trait;

use crate::bus::{
    Agent, AgentContext, AgentError, Command, CommandAuthorizer, Envelope, Event, RunCommand,
    RunEvent, Subscription, VoiceEvent,
};
use crate::state::RunStatus;
use crate::voice::{classify_spoken, SpokenIntent};

/// What a transcript routes to. The participant dispatches `RunCommand::Abort` for
/// `Cancel` through the bounded command intake (best-effort; swallowed on backpressure
/// — the physical control carries the deterministic abort guarantee).
/// `StartRun`/`Steer` map to a validated [`Command`] via [`IntakeAction::to_command`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntakeAction {
    /// Best-effort spoken cancel → run-cancel action.
    Cancel,
    /// No focused run → start a new run with this goal.
    StartRun { goal: String },
    /// A run is focused → steer it with this text.
    Steer { run_id: String, text: String },
}

/// Route a transcript given the currently focused run, if any (FR-006/FR-007).
///
/// `focused_run` reflects the state maintained reactively by the `Snapshot`/`Finished`
/// handlers in `handle()`. EC-007 (the focused run terminates while the utterance was
/// being transcribed) is handled by those handlers clearing `focused_run` when a run
/// becomes terminal — so by the time `act_on_transcript` reads it, a terminal run's id
/// has already been cleared and `None` is passed here without an explicit liveness check.
pub fn route_transcript(transcript: &str, focused_run: Option<&str>) -> IntakeAction {
    if classify_spoken(transcript) == SpokenIntent::Cancel {
        return IntakeAction::Cancel;
    }
    let text = transcript.trim().to_string();
    match focused_run {
        Some(run_id) => IntakeAction::Steer { run_id: run_id.to_string(), text },
        None => IntakeAction::StartRun { goal: text },
    }
}

impl IntakeAction {
    /// The validated [`Command`] for the free-form actions. `Cancel` returns `None`
    /// because the spoken cancel is dispatched directly by the caller via
    /// `ctx.dispatch(RunCommand::Abort, …)` in `act_on_transcript` (best-effort,
    /// swallowed on backpressure). Called by `act_on_transcript` for the
    /// `StartRun`/`Steer` arms so the mapping lives in exactly one place.
    pub fn to_command(&self) -> Option<Command> {
        match self {
            IntakeAction::Cancel => None,
            IntakeAction::StartRun { goal } => Some(Command::Run(RunCommand::Start { goal: goal.clone() })),
            IntakeAction::Steer { run_id, text } => {
                Some(Command::Run(RunCommand::Steer { run_id: run_id.clone(), text: text.clone() }))
            }
        }
    }
}

/// The intake participant: turns transcripts into bus actions. The capture→AEC→STT
/// half is device-gated and publishes a `voice.utterance_transcribed` event; this
/// Agent subscribes to it, applies the pure [`route_transcript`] decision, and acts
/// through the validated `dispatch` path. It also tracks the focused (live) run from
/// run snapshots so a free-form utterance knows whether to start or steer.
pub struct VoiceIntake {
    ctx: AgentContext,
    authz: Arc<dyn CommandAuthorizer>,
    /// The live run a free-form utterance steers, and the target of a spoken cancel.
    /// ponytail: single-session heuristic — the one Running/Paused run is the focus.
    /// Replace with an explicit UI focus signal when multi-run focus actually lands.
    focused_run: Option<String>,
}

impl VoiceIntake {
    pub fn new(ctx: AgentContext, authz: Arc<dyn CommandAuthorizer>) -> Self {
        Self { ctx, authz, focused_run: None }
    }

    /// The currently focused run, if any (test/observability aid).
    pub fn focused_run(&self) -> Option<&str> {
        self.focused_run.as_deref()
    }

    /// Route a transcript and act on the bus (FR-005a/006/007).
    fn act_on_transcript(&self, transcript: &str) -> Result<(), AgentError> {
        match route_transcript(transcript, self.focused_run.as_deref()) {
            IntakeAction::Cancel => {
                // Best-effort spoken cancel (council): dispatch `run.abort` for the
                // focused run (`None` ⇒ the single live session). A dispatch failure is
                // swallowed on purpose — the spoken path never promises delivery; the
                // *deterministic* abort guarantee lives on the physical control (US3).
                let _ = self.ctx.dispatch(
                    Command::Run(RunCommand::Abort { run_id: self.focused_run.clone() }),
                    self.authz.as_ref(),
                );
                Ok(())
            }
            // Free-form → a validated start/steer through the one authorized intake.
            // `to_command()` owns the action→Command mapping; to_command()'s exhaustive
            // match is the compile-time guard: adding a variant to IntakeAction without
            // updating to_command() is a compile error there.
            action => {
                let cmd = action.to_command().ok_or_else(|| {
                    AgentError::Other("IntakeAction variant returned no command".into())
                })?;
                self.ctx
                    .dispatch(cmd, self.authz.as_ref())
                    .map(|_| ())
                    .map_err(|e| {
                        let err = AgentError::Other(e.to_string());
                        eprintln!("[wagner] voice-intake: dispatch failed: {err}");
                        err
                    })
            }
        }
    }
}

#[async_trait]
impl Agent for VoiceIntake {
    fn name(&self) -> &str {
        "voice-intake"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![
            // Transcripts produced by the (device-gated) capture→STT pipeline.
            Subscription { topic: "voice".into(), filter: None },
            // Run snapshots/finishes — which run a free-form utterance steers (EC-007).
            Subscription { topic: "run".into(), filter: None },
        ]
    }

    async fn handle(&mut self, envelope: &Envelope) -> Result<(), AgentError> {
        match &envelope.payload {
            Event::Voice(VoiceEvent::UtteranceTranscribed { text }) => {
                self.act_on_transcript(text)?;
            }
            // Re-check liveness from the run's own state (EC-007): a live run becomes
            // the focus; a terminal snapshot of the focused run clears it.
            //
            // Safety of the unconditional first branch: the registry enforces a
            // single-session invariant — at most one run is Running/Paused at a time —
            // so two live snapshots for different runs cannot coexist. If that invariant
            // is ever relaxed, replace this with a guard: only set when focused_run is
            // None (ponytail: single-session heuristic; replace with an explicit UI focus
            // signal when multi-run focus lands).
            Event::Run(RunEvent::Snapshot(run)) => {
                if matches!(run.status, RunStatus::Running | RunStatus::Paused) {
                    self.focused_run = Some(run.run_id.clone());
                } else if self.focused_run.as_deref() == Some(run.run_id.as_str()) {
                    self.focused_run = None;
                }
            }
            Event::Run(RunEvent::Finished { run_id, .. }) => {
                if self.focused_run.as_deref() == Some(run_id.as_str()) {
                    self.focused_run = None;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_cancel_routes_to_cancel_regardless_of_focus() {
        assert_eq!(route_transcript("stop", Some("r1")), IntakeAction::Cancel);
        assert_eq!(route_transcript("never mind", None), IntakeAction::Cancel);
    }

    #[test]
    fn free_form_with_no_focus_starts_a_run() {
        assert_eq!(
            route_transcript("research the voice landscape", None),
            IntakeAction::StartRun { goal: "research the voice landscape".into() }
        );
    }

    #[test]
    fn free_form_with_focus_steers_that_run() {
        assert_eq!(
            route_transcript("use the other approach", Some("run-7")),
            IntakeAction::Steer { run_id: "run-7".into(), text: "use the other approach".into() }
        );
    }

    #[test]
    fn cancel_word_with_trailing_goal_is_not_a_cancel() {
        // "stop wasting tokens" is a steer/goal, never an abort (council safety case).
        assert_eq!(
            route_transcript("stop wasting tokens", Some("r1")),
            IntakeAction::Steer { run_id: "r1".into(), text: "stop wasting tokens".into() }
        );
        assert_eq!(
            route_transcript("stop wasting tokens", None),
            IntakeAction::StartRun { goal: "stop wasting tokens".into() }
        );
    }

    #[test]
    fn to_command_maps_free_form_and_skips_cancel() {
        assert_eq!(IntakeAction::Cancel.to_command(), None);
        assert_eq!(
            IntakeAction::StartRun { goal: "g".into() }.to_command(),
            Some(Command::Run(RunCommand::Start { goal: "g".into() }))
        );
        assert_eq!(
            IntakeAction::Steer { run_id: "r".into(), text: "t".into() }.to_command(),
            Some(Command::Run(RunCommand::Steer { run_id: "r".into(), text: "t".into() }))
        );
    }
}

#[cfg(test)]
mod agent_tests {
    use super::*;
    use crate::bus::{
        AgentRegistry, AllowAll, Bus, CommandEnvelope, NodeId, ParticipantId, ParticipantKind,
        StreamId,
    };
    use crate::state::Run;
    use tokio::sync::mpsc;

    fn pid() -> ParticipantId {
        ParticipantId {
            node: NodeId("local".into()),
            kind: ParticipantKind::Agent,
            name: "voice-intake".into(),
            instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        }
    }

    /// An intake plus the receiver draining the commands it dispatches (the bus
    /// command-intake channel — what `dispatch` enqueues, FR-008).
    fn intake_with_cmds() -> (VoiceIntake, mpsc::Receiver<CommandEnvelope>) {
        let bus = Arc::new(Bus::new(16));
        let cmds = bus.take_commands().expect("first take of command intake");
        let ctx = AgentRegistry::new(Arc::clone(&bus)).context(pid());
        (VoiceIntake::new(ctx, Arc::new(AllowAll)), cmds)
    }

    /// Mint a well-formed envelope around `payload` — only the payload matters to
    /// `handle` (the throwaway bus has no subscribers; the publish is dropped).
    fn env(payload: Event) -> Envelope {
        AgentRegistry::new(Arc::new(Bus::new(4)))
            .context(pid())
            .publish(StreamId::Workspace("test".into()), payload)
    }

    fn live_run(id: &str) -> Run {
        let mut r = Run::new(id.into(), "goal".into(), vec![], "2026-01-01T00:00:00Z".into());
        r.status = RunStatus::Running;
        r
    }

    fn transcribed(text: &str) -> Event {
        Event::Voice(VoiceEvent::UtteranceTranscribed { text: text.into() })
    }

    #[tokio::test]
    async fn no_focus_utterance_dispatches_start() {
        let (mut intake, mut cmds) = intake_with_cmds();
        intake.handle(&env(transcribed("research the landscape"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(cmd.command, Command::Run(RunCommand::Start { goal: "research the landscape".into() }));
    }

    #[tokio::test]
    async fn focused_utterance_dispatches_steer() {
        let (mut intake, mut cmds) = intake_with_cmds();
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(live_run("r7")))))).await.unwrap();
        assert_eq!(intake.focused_run(), Some("r7"), "a live snapshot sets the focus");
        intake.handle(&env(transcribed("use the other approach"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(
            cmd.command,
            Command::Run(RunCommand::Steer { run_id: "r7".into(), text: "use the other approach".into() })
        );
    }

    #[tokio::test]
    async fn spoken_cancel_dispatches_abort_for_focused_run() {
        let (mut intake, mut cmds) = intake_with_cmds();
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(live_run("r7")))))).await.unwrap();
        intake.handle(&env(transcribed("stop"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(cmd.command, Command::Run(RunCommand::Abort { run_id: Some("r7".into()) }));
    }

    #[tokio::test]
    async fn terminal_snapshot_clears_focus_so_next_utterance_starts() {
        // Path (b): a RunEvent::Snapshot with a non-live status clears focused_run (EC-007).
        let (mut intake, mut cmds) = intake_with_cmds();
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(live_run("r9")))))).await.unwrap();
        assert_eq!(intake.focused_run(), Some("r9"), "live snapshot sets the focus");
        // Send a terminal snapshot (Met) for the same run.
        let mut met = live_run("r9");
        met.status = RunStatus::Met;
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(met))))).await.unwrap();
        assert_eq!(intake.focused_run(), None, "terminal snapshot clears the focus (EC-007)");
        // Next utterance must dispatch Start, not Steer.
        intake.handle(&env(transcribed("brand new task"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(cmd.command, Command::Run(RunCommand::Start { goal: "brand new task".into() }));
    }

    #[tokio::test]
    async fn paused_snapshot_promotes_focus_and_steer_reaches_it() {
        // Exercises the `RunStatus::Paused` branch of the Snapshot handler —
        // a mutation changing `| RunStatus::Paused` to anything else should fail here.
        let (mut intake, mut cmds) = intake_with_cmds();
        let mut paused = live_run("r-paused");
        paused.status = RunStatus::Paused;
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(paused))))).await.unwrap();
        assert_eq!(intake.focused_run(), Some("r-paused"), "a Paused snapshot sets the focus");
        intake.handle(&env(transcribed("resume from checkpoint"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(
            cmd.command,
            Command::Run(RunCommand::Steer { run_id: "r-paused".into(), text: "resume from checkpoint".into() })
        );
    }

    #[tokio::test]
    async fn spoken_cancel_with_no_focus_dispatches_abort_run_id_none() {
        // Exercises the `focused_run = None` path: Abort { run_id: None } tells the
        // executor to abort the single live session (the "council" semantic).
        // A mutation replacing `self.focused_run.clone()` with `Some(hardcoded_id)`
        // would cause this assertion to fail.
        let (mut intake, mut cmds) = intake_with_cmds();
        // No Snapshot sent — focused_run stays None.
        intake.handle(&env(transcribed("stop"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(cmd.command, Command::Run(RunCommand::Abort { run_id: None }));
    }

    #[tokio::test]
    async fn finished_run_clears_focus_so_next_utterance_starts() {
        let (mut intake, mut cmds) = intake_with_cmds();
        intake.handle(&env(Event::Run(RunEvent::Snapshot(Box::new(live_run("r7")))))).await.unwrap();
        intake.handle(&env(Event::Run(RunEvent::Finished { run_id: "r7".into(), ok: true }))).await.unwrap();
        assert_eq!(intake.focused_run(), None, "a terminal finish clears the focus (EC-007)");
        intake.handle(&env(transcribed("new task"))).await.unwrap();
        let cmd = cmds.try_recv().expect("a command was dispatched");
        assert_eq!(cmd.command, Command::Run(RunCommand::Start { goal: "new task".into() }));
    }
}
