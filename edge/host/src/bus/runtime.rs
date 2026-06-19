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

use tokio::sync::broadcast;

use super::{Envelope, Event, StreamId, Subscription};

/// The in-process event bus. Cheap to `Arc`-share across tasks; `publish` is sync
/// and non-blocking (a slow subscriber lags, it does not back up the bus).
pub struct Bus {
    tx: broadcast::Sender<Envelope>,
    /// Next per-stream `seq` to assign. `publish` stamps the authoritative value.
    seqs: Mutex<HashMap<StreamId, u64>>,
}

impl Bus {
    /// Create a bus whose fan-out buffer holds `capacity` envelopes per subscriber
    /// before the slowest lags. `capacity` must be ≥ 1.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self { tx, seqs: Mutex::new(HashMap::new()) }
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
        Subscriber { rx: self.tx.subscribe(), subscription }
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
    subscription: Subscription,
}

impl Subscriber {
    /// Await the next matching envelope. Returns [`RecvError::Lagged`] once when the
    /// consumer fell behind the buffer (then recovers), or [`RecvError::Closed`]
    /// when the bus is dropped.
    pub async fn recv(&mut self) -> Result<Envelope, RecvError> {
        loop {
            match self.rx.recv().await {
                Ok(envelope) if matches(&self.subscription, &envelope) => return Ok(envelope),
                Ok(_) => continue, // delivered to the bus, filtered out for this subscription
                Err(broadcast::error::RecvError::Lagged(n)) => return Err(RecvError::Lagged(n)),
                Err(broadcast::error::RecvError::Closed) => return Err(RecvError::Closed),
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
