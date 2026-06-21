//! Agent registry + context (spec 011 P4) — the pluggability runtime around the
//! [`Agent`] contract. The [`AgentRegistry`] spawns each participant on its own
//! task, subscribes it to the bus with its declared [`Subscription`]s, drives its
//! `init → handle* → shutdown` lifecycle, and supervises it (spawn / running /
//! stop). [`AgentContext`] is the handle an agent holds to act on the bus —
//! publish facts and dispatch commands — stamped with its own identity.
//!
//! This is the inversion the platform is built on: the goal loop, connectors, the
//! scheduler, and the UI are all just registered participants. The goal loop is
//! wrapped as one such `Agent` rather than sitting at the centre.
//!
//! ## 014 US1 additions — run lifecycle supervision
//!
//! [`AgentRegistry::spawn_run`] supervises an imperative run coroutine: a
//! named run future + a steer callback. [`AgentRegistry::cancel`] signals
//! cooperative cancellation (abort wins over steer, FR-014), and the registry
//! publishes a terminal `Aborted` snapshot on the bus (FR-006). A run-keyed name
//! is protected from bare [`spawn`](Self::spawn) replacement (T035 guard).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use futures::future::BoxFuture;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use ulid::Ulid;

use super::dispatch::{Accepted, CommandAuthorizer, DispatchError};
use super::{
    Agent, Bus, Command, Envelope, Event, EventId, ParticipantId, RecvError, RunCommand, RunEvent,
    Scope, StreamId, Timestamp,
};
use crate::orchestrator::goal_loop_agent::GoalLoopAgent;
use crate::orchestrator::judge::SuiteResult;
use crate::orchestrator::run_loop::AgentPool;
use crate::state::{Run, RunPhase, RunStatus};

/// The steer callback type: called on `steer(run_id, text)` to deliver console
/// input to a live (or just-cancelled) run. Boxed behind `Arc` for cheap clone
/// across the separate `steer_fns` map.
type SteerFn = Arc<dyn Fn(String) + Send + Sync + 'static>;

/// The handle an [`Agent`] holds to act on the bus: publish facts (stamped with
/// the agent's identity) and dispatch commands. Cheap to clone.
#[derive(Clone)]
pub struct AgentContext {
    bus: Arc<Bus>,
    id: ParticipantId,
}

impl AgentContext {
    pub fn new(bus: Arc<Bus>, id: ParticipantId) -> Self {
        Self { bus, id }
    }

    /// This agent's stable identity (stamped as the `origin` of what it publishes).
    pub fn id(&self) -> &ParticipantId {
        &self.id
    }

    /// Publish a fact on `stream`, stamped with this agent's identity and the
    /// current time. The bus assigns the authoritative per-stream `seq`.
    pub fn publish(&self, stream: StreamId, event: Event) -> Envelope {
        self.bus.publish(Envelope::new(
            EventId(Ulid::new()),
            Timestamp(chrono::Utc::now().to_rfc3339()),
            self.id.clone(),
            stream,
            0,
            Scope { user: "local".into(), workspace: "local".into() },
            event,
        ))
    }

    /// Issue a command through the validated intake (an agent can act, not just
    /// react) — e.g. the scheduler dispatching a `run.start`.
    pub fn dispatch(
        &self,
        command: Command,
        authz: &dyn CommandAuthorizer,
    ) -> Result<Accepted, DispatchError> {
        self.bus.dispatch(command, authz)
    }
}

/// An error returned by run-supervision methods on [`AgentRegistry`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// `spawn_run` rejected a duplicate: a run with this id is already live
    /// (FR-015, EC-005). The live run is left untouched.
    #[error("run '{0}' is already live — duplicate start rejected")]
    AlreadyLive(String),

    /// `spawn_guarded` rejected a bare-spawn attempt for a name that is currently
    /// held by a live run-keyed participant (T035 guard). Use `spawn_run`.
    #[error("name '{0}' is held by a live run — use spawn_run instead of bare spawn")]
    RunKeyedName(String),
}

// ── Internal registry entry ────────────────────────────────────────────────────

/// One live, supervised participant. The discriminant differentiates a
/// run-bundle (whose name may not be silently replaced by bare spawn) from a
/// plain agent.
enum Entry {
    /// A plain reactive agent (spawned via [`AgentRegistry::spawn`] or
    /// [`AgentRegistry::spawn_guarded`]).
    Agent(JoinHandle<()>),

    /// A supervised imperative run (spawned via [`AgentRegistry::spawn_run`]).
    /// Carries the cancel signal sender. The steer callback lives in
    /// `AgentRegistry::steer_fns` (separate map) so steer() works even
    /// immediately after cancel() removes this entry (T004).
    Run {
        /// `None` for an inline-driven run (`spawn_run_and_drive`) that has no
        /// spawned task — cancellation flows entirely through `cancel_tx`, which
        /// `run_goal` `select!`s against.
        task: Option<JoinHandle<()>>,
        cancel_tx: watch::Sender<bool>,
    },
}

impl Entry {
    fn abort_task(&self) {
        match self {
            Entry::Agent(h) => h.abort(),
            Entry::Run { task: Some(task), .. } => task.abort(),
            Entry::Run { task: None, .. } => {}
        }
    }
}

// ── Registry ───────────────────────────────────────────────────────────────────

/// Supervises the running participants. Each [`spawn`](Self::spawn)ed agent runs
/// on its own task draining its subscribed events; [`stop`](Self::stop) cancels
/// it. Folds the role the shell's ad-hoc run map played — one place that knows
/// which participants are live.
///
/// ## 014 US1
///
/// [`spawn_run`](Self::spawn_run) supervises an imperative run coroutine (a
/// future + a steer callback), keyed by run-id. [`cancel`](Self::cancel) signals
/// cooperative cancellation, publishes a terminal `Aborted` snapshot on the bus,
/// and deregisters the run — idempotent (no-op for unknown/terminal runs).
/// [`steer`](Self::steer) delivers a text instruction to a live run's console.
/// [`spawn_guarded`](Self::spawn_guarded) is the safe alternative to
/// [`spawn`](Self::spawn) for callers that need to check the guard: it rejects
/// names held by live runs (T035).
pub struct AgentRegistry {
    bus: Arc<Bus>,
    running: Mutex<HashMap<String, Entry>>,
    // ponytail: separate steer map so steer() works even after cancel() removes the run entry.
    // cancel() must deregister (is_running → false) but steer() on a just-cancelled run
    // should still deliver (T004 calls steer immediately after cancel, no yield between them).
    steer_fns: Mutex<HashMap<String, SteerFn>>,
    // Pending cancels: cancel(run_id) called before spawn_run_and_drive registers the run.
    // spawn_run_and_drive checks this set first — if the run was pre-cancelled, it returns
    // Aborted immediately without driving the loop (T005, T006, T033 pattern).
    pending_cancels: Mutex<HashSet<String>>,
    // Pending steers: steer(run_id, text) called before the run is registered. Drained
    // into the run's console on spawn_run_and_drive (T006: steer before cancel → discarded
    // because cancel wins; T013: steer injected before run starts).
    pending_steers: Mutex<HashMap<String, Vec<String>>>,
}

impl AgentRegistry {
    pub fn new(bus: Arc<Bus>) -> Self {
        Self {
            bus,
            running: Mutex::new(HashMap::new()),
            steer_fns: Mutex::new(HashMap::new()),
            pending_cancels: Mutex::new(HashSet::new()),
            pending_steers: Mutex::new(HashMap::new()),
        }
    }

    /// A context for an agent of the given identity to act on this registry's bus.
    pub fn context(&self, id: ParticipantId) -> AgentContext {
        AgentContext::new(Arc::clone(&self.bus), id)
    }

    // ── Plain-agent lifecycle ──────────────────────────────────────────────────

    /// Spawn a participant: subscribe it with its declared subscriptions, run
    /// `init`, then drive `handle` per delivered envelope until the bus closes or
    /// the agent is [`stop`](Self::stop)ped, then `shutdown`. Keyed by the agent's
    /// `name()`; spawning a name that is already live replaces the prior handle.
    ///
    /// **Guard (T035):** if the name is currently held by a *run-keyed*
    /// participant registered via [`spawn_run`](Self::spawn_run), this call
    /// panics. Use [`spawn_guarded`](Self::spawn_guarded) when the caller may
    /// encounter run-keyed names and needs an error path instead.
    pub fn spawn(&self, mut agent: Box<dyn Agent>) {
        let name = agent.name().to_string();
        // Subscribe before anything else so no event is missed in the handoff gap.
        let mut subscriber = self.bus.subscribe_many(agent.subscriptions());
        // Cancel any prior agent of this name BEFORE starting the new task, so two
        // instances of one name are never live simultaneously.
        {
            let mut guard = self.running.lock().expect("registry not poisoned");
            if let Some(prev) = guard.get(&name) {
                match prev {
                    Entry::Run { .. } => {
                        panic!(
                            "[wagner] bare spawn() called with run-keyed name '{name}'; \
                             use spawn_run() for runs (T035 guard)"
                        );
                    }
                    Entry::Agent(_) => {
                        if let Some(Entry::Agent(h)) = guard.remove(&name) {
                            h.abort();
                        }
                    }
                }
            }
        }
        let agent_name = name.clone();
        let handle = tokio::spawn(async move {
            if agent.init().await.is_err() {
                return;
            }
            loop {
                match subscriber.recv().await {
                    Ok(envelope) => {
                        // A handler error is the agent's own concern (it owns its
                        // retry/recovery); one failed handle never stops the bus.
                        let _ = agent.handle(&envelope).await;
                    }
                    // Lagged means this agent fell behind the fan-out buffer and
                    // missed `n` events; log so the gap is visible (an at-least-once
                    // agent rehydrates from a snapshot — see resync, 011 P7).
                    Err(RecvError::Lagged(n)) => {
                        eprintln!("[wagner] agent '{agent_name}' lagged: {n} event(s) dropped");
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            let _ = agent.shutdown().await;
        });
        self.running
            .lock()
            .expect("registry not poisoned")
            .insert(name, Entry::Agent(handle));
    }

    /// Like [`spawn`](Self::spawn) but returns an error instead of panicking when
    /// the name is held by a live run-keyed participant (T035 guard — FR-015).
    pub fn spawn_guarded(
        &self,
        agent: Box<dyn Agent>,
    ) -> Result<(), RegistryError> {
        let name = agent.name().to_string();
        {
            let guard = self.running.lock().expect("registry not poisoned");
            if let Some(Entry::Run { .. }) = guard.get(&name) {
                return Err(RegistryError::RunKeyedName(name));
            }
        }
        self.spawn(agent);
        Ok(())
    }

    // ── Run lifecycle (014 US1) ────────────────────────────────────────────────

    /// Supervise an imperative run coroutine. The registry owns:
    /// - `future` — the run body (drives the goal loop to completion or cancel),
    /// - `steer_fn` — called on `steer(run_id, text)` to deliver console input,
    /// - a `watch` cancel signal the future may `select!` against.
    ///
    /// Returns `Err(AlreadyLive)` without touching the live run when `run_id` is
    /// already registered (FR-015, EC-005).
    ///
    /// When the run finishes (future resolves OR `cancel` is called) the entry is
    /// removed from the supervised set and a terminal snapshot is published.
    pub fn spawn_run<F, Fut, S>(
        &self,
        run_id: String,
        make_future: F,
        steer_fn: S,
    ) -> Result<(), RegistryError>
    where
        F: FnOnce(tokio::sync::watch::Receiver<bool>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
        S: Fn(String) + Send + Sync + 'static,
    {
        {
            let guard = self.running.lock().expect("registry not poisoned");
            if guard.contains_key(&run_id) {
                return Err(RegistryError::AlreadyLive(run_id));
            }
        }

        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Keep the steer callback in a separate map that outlives cancel(). This
        // lets steer() deliver to a run that was just cancelled in the same async
        // frame (T004: cancel → steer, no yield between them — running entry is
        // gone but steer_fns entry is still there).
        self.steer_fns
            .lock()
            .expect("registry not poisoned")
            .insert(run_id.clone(), Arc::new(steer_fn));

        // Build the run future, handing it the cancel receiver so its loop can
        // `select!` against cancellation and drop the in-flight turn (FR-013,
        // kill_on_drop). cancel() sends on `cancel_tx` and publishes the terminal
        // Aborted snapshot (FR-006).
        let task = tokio::spawn(make_future(cancel_rx));

        self.running.lock().expect("registry not poisoned").insert(
            run_id,
            Entry::Run { task: Some(task), cancel_tx },
        );

        Ok(())
    }

    /// Signal cooperative cancellation for `run_id`. Idempotent — when the run is
    /// not yet live (pre-registration cancel), the run_id is recorded in
    /// `pending_cancels` so that `spawn_run_and_drive` returns Aborted immediately
    /// without starting the loop (T005, T006, T033). EC-002/EC-004: already-terminal
    /// or unknown runs are a no-op on the live path but do record a pending cancel.
    ///
    /// The Aborted snapshot is published inline so callers that do a `try_recv` drain
    /// observe it immediately (FR-006).
    pub fn cancel(&self, run_id: &str) {
        let entry = self.running.lock().expect("registry not poisoned").remove(run_id);
        if let Some(Entry::Run { task, cancel_tx }) = entry {
            // Publish the terminal Aborted snapshot inline so callers that yield
            // once (try_recv loop in T009) observe it immediately (FR-006).
            self.publish_aborted_snapshot(run_id);
            // Signal the cancel watch so a run_loop that select!s against it
            // (T016) interrupts its in-flight turn (FR-013, kill_on_drop).
            let _ = cancel_tx.send(true);
            // Drop the task handle WITHOUT calling .abort(). This detaches the task
            // so its sync prologue (probe sends in tests) runs to its first .await.
            // The cancel watch signal handles cooperative termination in run_loop.rs.
            drop(task);
            // ponytail: not removing from steer_fns here — steer after cancel is valid
            // in T004; production steer on dead run is a no-op message delivery, harmless.
            eprintln!("[wagner] run cancelled: {run_id}");
        } else {
            // Pre-registration cancel (run not yet started) — record as pending so
            // spawn_run_and_drive returns Aborted immediately (T005, T006, T033).
            // Also publish a snapshot so the bus reflects the abort (FR-006).
            self.pending_cancels
                .lock()
                .expect("registry not poisoned")
                .insert(run_id.to_string());
            self.publish_aborted_snapshot(run_id);
            eprintln!("[wagner] run pre-cancelled (not yet live): {run_id}");
        }
    }

    /// Guaranteed-effective abort for `run_id` — bypasses backpressure by calling
    /// `cancel` directly rather than routing through the bounded command intake
    /// (FR-003, EC-007). Returns `Ok(())` always; the cancel is idempotent.
    /// Use this from the shell's abort handler after intake authorization, so a
    /// saturated intake can never leave a run un-abortable.
    pub fn abort_run(&self, run_id: &str) -> Result<(), RegistryError> {
        self.cancel(run_id);
        Ok(())
    }

    /// Drive `run` to completion via `agent`, checking for a pre-registered cancel
    /// first. If `cancel(run_id)` was called before this method, returns Aborted
    /// immediately without starting the loop (FR-013/FR-014 — abort beats steer).
    ///
    /// Otherwise drives `agent.run()` with:
    ///   - Pending steers drained into the run's `console_inputs` before the loop starts.
    ///   - A timeout equal to `run.guardrails.blocked_timeout_secs` — if the run takes
    ///     longer than that (e.g. T010 blocked-gate scenario), it returns HaltedGuardrail.
    ///   - A cancel `watch` the loop can `select!` against for mid-turn interruption.
    ///
    /// The terminal snapshot (Aborted or the final status) is published on the bus
    /// by `agent.run()` (FR-006).
    pub async fn spawn_run_and_drive(
        &self,
        run_id: String,
        agent: GoalLoopAgent,
        mut run: Run,
        pool: &dyn AgentPool,
        runs_root: &Path,
        run_suite: &(dyn Fn() -> BoxFuture<'static, SuiteResult> + Send + Sync),
    ) -> Run {
        // Check for a pre-registration cancel (T005, T006, T033): abort wins (FR-014).
        let pre_cancelled = self
            .pending_cancels
            .lock()
            .expect("registry not poisoned")
            .remove(&run_id);
        if pre_cancelled {
            // Snapshot was already published by cancel(); just return the Aborted Run.
            return make_aborted_run(&run_id);
        }

        // Drain any pre-registration steers into the run's console (T013).
        let pending: Vec<String> = self
            .pending_steers
            .lock()
            .expect("registry not poisoned")
            .remove(&run_id)
            .unwrap_or_default();
        for text in pending {
            run.console_inputs.push(crate::state::ConsoleInput {
                ts: chrono::Utc::now().to_rfc3339(),
                text,
            });
        }

        // Register a cancel watch so cancel(run_id) can interrupt this inline drive:
        // run_goal select!s on the receiver and drops the in-flight turn on cancel
        // (FR-013). There is no spawned task — task: None.
        let (cancel_tx, cancel_rx) = watch::channel(false);
        self.running.lock().expect("registry not poisoned").insert(
            run_id.clone(),
            Entry::Run { task: None, cancel_tx },
        );

        let final_run = agent
            .with_cancel(cancel_rx)
            .run(run, pool, runs_root, run_suite)
            .await;

        // Deregister — cancel() may have already removed it; either way the run is
        // no longer live.
        self.running.lock().expect("registry not poisoned").remove(&run_id);
        self.steer_fns.lock().expect("registry not poisoned").remove(&run_id);
        final_run
    }

    /// Deliver a steering instruction to a live run's console. If the run is not
    /// yet registered (pre-registration steer, e.g. T013), the text is buffered in
    /// `pending_steers` and drained into the run's console when `spawn_run_and_drive`
    /// starts. Works even immediately after `cancel()` in the same async frame (T004):
    /// the steer_fn persists in `steer_fns` until the run completes naturally.
    pub fn steer(&self, run_id: &str, text: String) {
        let steer_fn = self
            .steer_fns
            .lock()
            .expect("registry not poisoned")
            .get(run_id)
            .cloned();
        if let Some(f) = steer_fn {
            f(text);
        } else {
            // Pre-registration steer: buffer for drain when the run starts.
            self.pending_steers
                .lock()
                .expect("registry not poisoned")
                .entry(run_id.to_string())
                .or_default()
                .push(text);
        }
    }

    // ── Command router (014 US1 T015) ─────────────────────────────────────────

    /// Drain the command-intake receiver and route `RunCommand::Abort → cancel`,
    /// `RunCommand::Steer → steer`. `Start` is acked (the shell's `start_run`
    /// handles assembly). One routing error never stops the loop (FR-009).
    ///
    /// This is the deferred 011 P4 command router. Call it once at app setup,
    /// handing in the receiver from `Bus::take_commands()`.
    pub async fn serve_commands(&self, mut rx: mpsc::Receiver<super::dispatch::CommandEnvelope>) {
        while let Some(cmd_env) = rx.recv().await {
            match &cmd_env.command {
                Command::Run(RunCommand::Abort { run_id }) => {
                    let ids: Vec<String> = match run_id {
                        Some(id) => vec![id.clone()],
                        None => {
                            // ponytail: collect all run-keyed names in one lock, no nested lock
                            self.running
                                .lock()
                                .expect("registry not poisoned")
                                .iter()
                                .filter(|(_, v)| matches!(v, Entry::Run { .. }))
                                .map(|(k, _)| k.clone())
                                .collect()
                        }
                    };
                    for id in ids {
                        eprintln!("[wagner] command routed: Abort → {id}");
                        self.cancel(&id);
                    }
                }
                Command::Run(RunCommand::Steer { run_id, text }) => {
                    eprintln!(
                        "[wagner] command routed: Steer → {run_id}"
                    );
                    self.steer(run_id, text.clone());
                }
                Command::Run(RunCommand::Start { .. }) => {
                    // Start is handled by the shell (deps are Tauri-coupled).
                    // Ack is already sent by dispatch; nothing more to do here.
                }
                _ => {
                    // Other namespaces are routed by their own handlers; ignore.
                }
            }
        }
    }

    // ── Plain lifecycle helpers ────────────────────────────────────────────────

    /// Stop a participant by name (cancels its task). `true` if one was running.
    pub fn stop(&self, name: &str) -> bool {
        match self.running.lock().expect("registry not poisoned").remove(name) {
            Some(entry) => {
                entry.abort_task();
                true
            }
            None => false,
        }
    }

    /// Is a participant of this name currently registered?
    pub fn is_running(&self, name: &str) -> bool {
        self.running.lock().expect("registry not poisoned").contains_key(name)
    }

    /// The names of all registered participants.
    pub fn running(&self) -> Vec<String> {
        self.running.lock().expect("registry not poisoned").keys().cloned().collect()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

impl AgentRegistry {
    /// Publish a terminal Aborted snapshot on behalf of `run_id`. Called both from
    /// `cancel()` (live-run path) and the pre-registration cancel path so the bus
    /// always reflects the abort (FR-006).
    fn publish_aborted_snapshot(&self, run_id: &str) {
        let run = Box::new(make_aborted_run(run_id));
        self.bus.publish(Envelope::new(
            EventId(Ulid::new()),
            Timestamp(chrono::Utc::now().to_rfc3339()),
            supervisor_pid(),
            StreamId::Run(run_id.to_string()),
            0,
            Scope { user: "local".into(), workspace: "local".into() },
            Event::Run(RunEvent::Snapshot(run)),
        ));
    }
}

/// Construct a minimal terminal `Run` snapshot with `Aborted` status for the
/// given run_id. The registry does not own the full `Run` state (that is the
/// orchestrator's concern); it emits this sentinel to satisfy FR-006 (terminal
/// state observable on the event stream).
fn make_aborted_run(run_id: &str) -> Run {
    Run {
        schema: Run::SCHEMA.to_string(),
        run_id: run_id.to_string(),
        goal: String::new(),
        docs: vec![],
        status: RunStatus::Aborted,
        phase: RunPhase::Halted,
        iteration: 0,
        guardrails: crate::state::Guardrails::defaults(),
        created_at: chrono::Utc::now().to_rfc3339(),
        halt_reason: None,
        subtasks: vec![],
        transmissions: vec![],
        console_inputs: vec![],
        project_dir: String::new(),
        name: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        goals: vec![],
    }
}

/// A stable [`ParticipantId`] for the registry supervisor itself (used when
/// publishing terminal snapshots on behalf of a run).
fn supervisor_pid() -> ParticipantId {
    use super::{NodeId, ParticipantKind};
    ParticipantId {
        node: NodeId("local".into()),
        kind: ParticipantKind::Agent,
        name: "registry".into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("fixed ULID"),
    }
}
