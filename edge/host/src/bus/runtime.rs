//! The in-process event bus (spec 011 P1) — the spine that carries [`Envelope`]s
//! from publishers to subscribers. **Behavioural**, unlike the rest of this module
//! (the contracts are pure data); built and tested standalone before anything in
//! the app depends on it (strangler-fig, `specs/011` plan Step 1).
//!
//! v1 is a `tokio::broadcast` fan-out: every subscriber sees every published
//! envelope, filtered locally by its [`Subscription`]. The bus is the authority
//! for per-stream `seq` (monotonic per [`StreamId`]). Slow subscribers surface a
//! [`RecvError::Lagged`] (oldest envelopes dropped) and then recover — the bus is
//! never blocked by one slow consumer.
//!
//! Not yet here (later plan steps, additive): the bounded `mpsc` command-intake
//! path (`dispatch`, P3) and the `watch` latest-state channel (P7 snapshot) — they
//! land with the consumers that need them, not speculatively.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::{broadcast, mpsc};

use super::dispatch::{Accepted, CommandAuthorizer, CommandEnvelope, DispatchError};
use super::{Command, Envelope, Event, EventId, StreamId, Subscription};

/// The command-intake schema (the exported catalog entry), embedded for boundary
/// validation of JSON commands (FR-015 / 011 P3).
const COMMAND_SCHEMA: &str = include_str!("../../schemas/bus/command.json");

/// The in-process event bus. Cheap to `Arc`-share across tasks; `publish` is sync
/// and non-blocking (a slow subscriber lags, it does not back up the bus).
pub struct Bus {
    tx: broadcast::Sender<Envelope>,
    /// Next per-stream `seq` to assign. `publish` stamps the authoritative value.
    seqs: Mutex<HashMap<StreamId, u64>>,
    /// Bounded command intake (011 P3): `dispatch` enqueues here; the registry
    /// (011 P4) claims the receiver via [`Bus::take_commands`].
    cmd_tx: mpsc::Sender<CommandEnvelope>,
    cmd_rx: Mutex<Option<mpsc::Receiver<CommandEnvelope>>>,
}

impl Bus {
    /// Create a bus whose fan-out buffer holds `capacity` envelopes per subscriber
    /// before the slowest lags, and whose command intake buffers `capacity`
    /// commands before applying backpressure. `capacity` must be ≥ 1.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        let (cmd_tx, cmd_rx) = mpsc::channel(capacity.max(1));
        Self {
            tx,
            seqs: Mutex::new(HashMap::new()),
            cmd_tx,
            cmd_rx: Mutex::new(Some(cmd_rx)),
        }
    }

    /// Intake one command (011 P3): **validate** (by construction — a typed
    /// `Command` is well-formed), **authorize** (Article IX), **stamp** an id, and
    /// **enqueue** for the registry. The single validated, authorized way to act.
    pub fn dispatch(
        &self,
        command: Command,
        authz: &dyn CommandAuthorizer,
    ) -> Result<Accepted, DispatchError> {
        authz.authorize(&command).map_err(DispatchError::Denied)?;
        let id = EventId(ulid::Ulid::new());
        let envelope = CommandEnvelope { id: id.clone(), command };
        match self.cmd_tx.try_send(envelope) {
            Ok(()) => Ok(Accepted { id }),
            Err(mpsc::error::TrySendError::Full(_)) => Err(DispatchError::Backpressure),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(DispatchError::NoConsumer),
        }
    }

    /// Intake a command that arrived as JSON (e.g. from a plugin or the frontend):
    /// **validate against the command schema** at the boundary (FR-015), then
    /// deserialize and [`dispatch`](Self::dispatch). Schema-invalid input is
    /// rejected before it can become a typed `Command`.
    pub fn dispatch_json(
        &self,
        raw: &serde_json::Value,
        authz: &dyn CommandAuthorizer,
    ) -> Result<Accepted, DispatchError> {
        crate::schema::validate(COMMAND_SCHEMA, raw)
            .map_err(|e| DispatchError::Invalid(e.to_string()))?;
        let command: Command = serde_json::from_value(raw.clone())
            .map_err(|e| DispatchError::Invalid(e.to_string()))?;
        self.dispatch(command, authz)
    }

    /// Claim the command-intake receiver (once). The registry (011 P4) calls this
    /// to drain dispatched commands; returns `None` if already taken.
    pub fn take_commands(&self) -> Option<mpsc::Receiver<CommandEnvelope>> {
        self.cmd_rx.lock().expect("bus cmd_rx not poisoned").take()
    }

    /// Stamp the authoritative per-stream `seq` (0-based, monotonic per
    /// [`StreamId`]), fan the envelope out to every subscriber, and return the
    /// stamped envelope. Sending with no live subscribers is not an error — the
    /// envelope is simply dropped.
    pub fn publish(&self, mut envelope: Envelope) -> Envelope {
        let seq = {
            let mut seqs = self.seqs.lock().expect("bus seq map not poisoned");
            let next = seqs.entry(envelope.stream.clone()).or_insert(0);
            let assigned = *next;
            *next += 1;
            assigned
        };
        envelope.seq = seq;
        // Err only means no subscribers; that is fine for a fan-out bus.
        let _ = self.tx.send(envelope.clone());
        envelope
    }

    /// Subscribe with a topic/namespace filter. The returned [`Subscriber`] only
    /// sees envelopes from publishes made *after* this call (broadcast semantics).
    pub fn subscribe(&self, subscription: Subscription) -> Subscriber {
        Subscriber { rx: self.tx.subscribe(), subscriptions: vec![subscription] }
    }

    /// Subscribe with several filters at once (a participant declares many). The
    /// subscriber delivers an envelope matching *any* of `subscriptions` (011 P4:
    /// the registry subscribes each agent with its full subscription set).
    pub fn subscribe_many(&self, subscriptions: Vec<Subscription>) -> Subscriber {
        Subscriber { rx: self.tx.subscribe(), subscriptions }
    }

    /// Number of live subscribers (test/observability aid).
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

/// A filtered view of the bus stream. `recv` yields only envelopes matching the
/// [`Subscription`]; a [`RecvError::Lagged`] surfaces dropped-envelope counts for a
/// slow consumer, after which `recv` resumes from the oldest retained envelope.
pub struct Subscriber {
    rx: broadcast::Receiver<Envelope>,
    subscriptions: Vec<Subscription>,
}

impl Subscriber {
    /// Await the next matching envelope. Returns [`RecvError::Lagged`] once when the
    /// consumer fell behind the buffer (then recovers), or [`RecvError::Closed`]
    /// when the bus is dropped.
    pub async fn recv(&mut self) -> Result<Envelope, RecvError> {
        loop {
            match self.rx.recv().await {
                Ok(envelope) if self.subscriptions.iter().any(|s| matches(s, &envelope)) => {
                    return Ok(envelope)
                }
                Ok(_) => continue, // delivered to the bus, filtered out for these subscriptions
                Err(broadcast::error::RecvError::Lagged(n)) => return Err(RecvError::Lagged(n)),
                Err(broadcast::error::RecvError::Closed) => return Err(RecvError::Closed),
            }
        }
    }

    /// Non-blocking: the next matching buffered envelope, or `None` when the
    /// buffer holds no (more) matching envelopes right now. Skips filtered and
    /// lagged-over envelopes. Useful for draining already-published facts.
    pub fn try_recv(&mut self) -> Option<Envelope> {
        loop {
            match self.rx.try_recv() {
                Ok(envelope) if self.subscriptions.iter().any(|s| matches(s, &envelope)) => {
                    return Some(envelope)
                }
                Ok(_) => continue,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Empty)
                | Err(broadcast::error::TryRecvError::Closed) => return None,
            }
        }
    }
}

/// Why a [`Subscriber::recv`] did not return an envelope.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecvError {
    /// The consumer fell behind the fan-out buffer; `n` envelopes were dropped.
    /// The next `recv` resumes from the oldest retained envelope.
    #[error("subscriber lagged; {0} envelope(s) dropped")]
    Lagged(u64),
    /// The bus (all `Sender`s) was dropped; the stream is finished.
    #[error("bus closed")]
    Closed,
}

/// The core-`Event` namespace of an envelope, allocation-free.
fn namespace_of(event: &Event) -> &'static str {
    match event {
        Event::Run(_) => "run",
        Event::Goal(_) => "goal",
        Event::Vault(_) => "vault",
        Event::Voice(_) => "voice",
        Event::Ui(_) => "ui",
        Event::Ext { .. } => "ext",
    }
}

/// The stream's discriminant value (the inner id), for `topic = "stream"` subs.
fn stream_value(stream: &StreamId) -> &str {
    match stream {
        StreamId::Run(v) | StreamId::Agent(v) | StreamId::Workspace(v) => v,
    }
}

/// Does this subscription match this envelope? (FR-011 matching, plan Step 1.)
///
/// - `topic = "*"` → everything.
/// - `topic = "stream"` → match when `filter` equals the envelope's stream value.
/// - `topic = "ext.<ns>"` → match an `Event::Ext` in `<ns>`; `filter` is the event
///   `name` (`None`/`"*"` = any name in that ext namespace).
/// - `topic = <namespace>` (run/goal/vault/voice/ui) → match that namespace.
///
// ponytail: core-namespace matching is namespace-granular (filter is ignored for
// core topics); leaf-level core filtering lands when a subscriber needs it. Ext
// already filters by name, which is where fine-grained selection is actually used.
fn matches(sub: &Subscription, env: &Envelope) -> bool {
    let topic = sub.topic.as_str();
    if topic == "*" {
        return true;
    }
    if topic == "stream" {
        return sub.filter.as_deref() == Some(stream_value(&env.stream));
    }
    if let Some(ext_ns) = topic.strip_prefix("ext.") {
        return match &env.payload {
            Event::Ext { ns, name, .. } => ns == ext_ns && filter_name(&sub.filter, name),
            _ => false,
        };
    }
    topic == namespace_of(&env.payload)
}

/// `None`/`Some("*")` match any name; otherwise an exact-name match.
fn filter_name(filter: &Option<String>, name: &str) -> bool {
    match filter.as_deref() {
        None | Some("*") => true,
        Some(f) => f == name,
    }
}
