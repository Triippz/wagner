//! Voice projection participant (spec 015, US4) — speaks an **allowlist** of bus
//! events aloud via the existing `Tts` port, and stays silent on everything else
//! (FR-009). The audio-playing `Agent` impl (subscribe → `Tts` → cpal playback)
//! lands with the device layer; the **speak policy** below is pure, deterministic,
//! and headless-testable so the allowlist is verified without any audio.
//!
//! Allowlist (FR-009), mapped onto the current `RunEvent` taxonomy:
//! - `Finished { ok: true }`  → "Run complete."
//! - `Finished { ok: false }` → "Run stopped."        (stopped / aborted)
//! - `Transmission`           → "Waiting for your approval."  (gate-blocked / awaiting answer)
//!
//! Everything else is silent. The set is **additive-versioned**: when an explicit
//! assistant-conversation event lands, it joins the match below — no rework.

use std::sync::Arc;

use async_trait::async_trait;

use crate::bus::{Agent, AgentError, Envelope, Event, RunEvent, Subscription};
use crate::voice::tts::Tts;

/// The projection allowlist (FR-009): the spoken text for a speakable event, or
/// `None` to stay silent. This is the single source of truth for what voice says.
pub fn speakable_text(event: &Event) -> Option<String> {
    match event {
        Event::Run(RunEvent::Finished { ok: true, .. }) => Some("Run complete.".to_string()),
        Event::Run(RunEvent::Finished { ok: false, .. }) => Some("Run stopped.".to_string()),
        Event::Run(RunEvent::Transmission(_)) => Some("Waiting for your approval.".to_string()),
        _ => None,
    }
}

/// The US4 projection participant: subscribes to the run namespace and speaks the
/// allowlisted events (FR-009) through the injected [`Tts`] port; silent on
/// everything else (SC-004). Handing the synthesised [`SpeechChunk`] to cpal
/// **playback** is the device-gated half (T038/T008) — this Agent stops at
/// synthesis, which is the full headless-testable surface of the allowlist.
pub struct VoiceProjection {
    tts: Arc<dyn Tts>,
}

impl VoiceProjection {
    pub fn new(tts: Arc<dyn Tts>) -> Self {
        Self { tts }
    }
}

#[async_trait]
impl Agent for VoiceProjection {
    fn name(&self) -> &str {
        "voice-projection"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        // The whole run namespace; `speakable_text` is the leaf allowlist (FR-009).
        vec![Subscription { topic: "run".into(), filter: None }]
    }

    async fn handle(&mut self, envelope: &Envelope) -> Result<(), AgentError> {
        if let Some(text) = speakable_text(&envelope.payload) {
            // ponytail: synthesise only — playback (cpal) is the device-gated half (T038).
            self.tts
                .synthesise(&text)
                .await
                .map_err(|e| AgentError::Other(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{GoalEvent, UiEvent, VaultEvent, VoiceEvent};
    use serde_json::json;

    fn finished(ok: bool) -> Event {
        Event::Run(RunEvent::Finished { run_id: "r1".into(), ok })
    }

    #[test]
    fn run_complete_speaks() {
        assert_eq!(speakable_text(&finished(true)).as_deref(), Some("Run complete."));
    }

    #[test]
    fn run_stopped_speaks() {
        assert_eq!(speakable_text(&finished(false)).as_deref(), Some("Run stopped."));
    }

    #[test]
    fn awaiting_approval_speaks() {
        let ev = Event::Run(RunEvent::Transmission(json!({ "tool": "Bash" })));
        assert_eq!(speakable_text(&ev).as_deref(), Some("Waiting for your approval."));
    }

    #[test]
    fn non_allowlisted_events_are_silent() {
        // A representative spread of the firehose — none of these is spoken (SC-004).
        let silent = [
            Event::Goal(GoalEvent::Added { goal_id: "g1".into(), title: "x".into() }),
            Event::Vault(VaultEvent::NoteUpdated { path: "n.md".into(), rev: 1 }),
            Event::Ui(UiEvent::SurfaceFocused { surface: "console".into() }),
            Event::Voice(VoiceEvent::UtteranceTranscribed { text: "hello".into() }),
            Event::Run(RunEvent::WorkflowStep(json!({}))),
            Event::Run(RunEvent::WorkflowDone(json!({}))),
        ];
        for ev in silent {
            assert_eq!(speakable_text(&ev), None, "{ev:?} must be silent");
        }
    }
}

#[cfg(test)]
mod agent_tests {
    use super::*;
    use crate::bus::{AgentContext, AgentRegistry, Bus, NodeId, ParticipantId, ParticipantKind, StreamId};
    use crate::voice::types::{SpeechChunk, VoiceError};
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A `Tts` that counts synthesis calls (the shipped `FakeTts` doesn't), so the
    /// allowlist gate is asserted by *invocation count* — the SC-004 contract.
    struct CountingTts {
        calls: Arc<AtomicU32>,
    }
    #[async_trait]
    impl Tts for CountingTts {
        async fn synthesise(&self, text: &str) -> Result<SpeechChunk, VoiceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(SpeechChunk::new(text.as_bytes().to_vec(), text))
        }
    }

    fn ctx(bus: &Arc<Bus>) -> AgentContext {
        AgentRegistry::new(Arc::clone(bus)).context(ParticipantId {
            node: NodeId("local".into()),
            kind: ParticipantKind::Agent,
            name: "voice-projection".into(),
            instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        })
    }

    #[tokio::test]
    async fn speaks_each_allowlisted_event() {
        let calls = Arc::new(AtomicU32::new(0));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let mut proj = VoiceProjection::new(Arc::new(CountingTts { calls: Arc::clone(&calls) }));

        for ev in [
            Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: true }),
            Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: false }),
            Event::Run(RunEvent::Transmission(json!({ "tool": "Bash" }))),
        ] {
            proj.handle(&ctx.publish(StreamId::Run("r1".into()), ev)).await.unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3, "all three allowlisted events spoke");
    }

    #[tokio::test]
    async fn silent_on_non_allowlisted_events() {
        let calls = Arc::new(AtomicU32::new(0));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let mut proj = VoiceProjection::new(Arc::new(CountingTts { calls: Arc::clone(&calls) }));

        for ev in [
            Event::Run(RunEvent::WorkflowStep(json!({}))),
            Event::Run(RunEvent::WorkflowDone(json!({}))),
        ] {
            proj.handle(&ctx.publish(StreamId::Run("r1".into()), ev)).await.unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0, "non-allowlisted run events stay silent (SC-004)");
    }
}
