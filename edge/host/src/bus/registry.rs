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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;
use ulid::Ulid;

use super::dispatch::{Accepted, CommandAuthorizer, DispatchError};
use super::{
    Agent, Bus, Command, Envelope, Event, EventId, ParticipantId, RecvError, Scope, StreamId,
    Timestamp,
};

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

/// Supervises the running participants. Each [`spawn`](Self::spawn)ed agent runs
/// on its own task draining its subscribed events; [`stop`](Self::stop) cancels
/// it. Folds the role the shell's ad-hoc run map played — one place that knows
/// which participants are live.
pub struct AgentRegistry {
    bus: Arc<Bus>,
    running: Mutex<HashMap<String, JoinHandle<()>>>,
}

impl AgentRegistry {
    pub fn new(bus: Arc<Bus>) -> Self {
        Self { bus, running: Mutex::new(HashMap::new()) }
    }

    /// A context for an agent of the given identity to act on this registry's bus.
    pub fn context(&self, id: ParticipantId) -> AgentContext {
        AgentContext::new(Arc::clone(&self.bus), id)
    }

    /// Spawn a participant: subscribe it with its declared subscriptions, run
    /// `init`, then drive `handle` per delivered envelope until the bus closes or
    /// the agent is [`stop`](Self::stop)ped, then `shutdown`. Keyed by the agent's
    /// `name()`; spawning a name that is already live replaces the prior handle.
    pub fn spawn(&self, mut agent: Box<dyn Agent>) {
        let name = agent.name().to_string();
        // Subscribe before anything else so no event is missed in the handoff gap.
        let mut subscriber = self.bus.subscribe_many(agent.subscriptions());
        // Cancel any prior agent of this name BEFORE starting the new task, so two
        // instances of one name are never live simultaneously.
        if let Some(prev) = self.running.lock().expect("registry not poisoned").remove(&name) {
            prev.abort();
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
        self.running.lock().expect("registry not poisoned").insert(name, handle);
    }

    /// Stop a participant by name (cancels its task). `true` if one was running.
    pub fn stop(&self, name: &str) -> bool {
        match self.running.lock().expect("registry not poisoned").remove(name) {
            Some(handle) => {
                handle.abort();
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
