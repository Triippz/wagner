//! Voice projection participant (spec 015, US4) — speaks an **allowlist** of bus
//! events aloud via the existing `Tts` port, and stays silent on everything else
//! (FR-009). The `Agent` impl (subscribe → allowlist filter → `Tts` synthesis) is
//! headless-testable and lives here; the cpal playback of the synthesised
//! `SpeechChunk` is device-gated (T038/T008). The **speak policy**
//! (`speakable_text`) is pure and deterministic.
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
use crate::voice::manager::VoiceManager;
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
    /// The live toggle + error sink (FR-015 / FR-014). Off ⇒ stay silent even
    /// while run events flow; a TTS failure is surfaced via `report_error`.
    voice: Arc<VoiceManager>,
}

impl VoiceProjection {
    pub fn new(tts: Arc<dyn Tts>, voice: Arc<VoiceManager>) -> Self {
        Self { tts, voice }
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
        // FR-015 toggle gate: voice off ⇒ silent, even though run events keep flowing.
        if !self.voice.enabled() {
            return Ok(());
        }
        if let Some(text) = speakable_text(&envelope.payload) {
            // ponytail: synthesise only — playback (cpal) is the device-gated half (T038).
            if let Err(e) = self.tts.synthesise(&text).await {
                // FR-014: surface the typed TTS-down error via VoiceStatus (no panic).
                // `e` is already a `VoiceError` (often `TtsFailed`) — report it verbatim,
                // and leave a stderr breadcrumb for device-side debugging.
                eprintln!("[wagner] voice-projection: tts synthesis failed: {e}");
                self.voice.report_error(&e);
            }
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
    use std::sync::Mutex;

    /// A `Tts` that counts synthesis calls — used to assert the SC-004 silence contract.
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

    /// A `Tts` that captures every synthesised phrase in order, so the round-trip
    /// through `VoiceProjection::handle` can be asserted phrase-by-phrase (not just
    /// by call count). A mutation that swaps phrases would be caught here.
    struct CapturingTts {
        phrases: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait]
    impl Tts for CapturingTts {
        async fn synthesise(&self, text: &str) -> Result<SpeechChunk, VoiceError> {
            self.phrases.lock().unwrap().push(text.to_string());
            Ok(SpeechChunk::new(text.as_bytes().to_vec(), text))
        }
    }

    /// A `Tts` that always fails — exercises the FR-014 error-surfacing path.
    struct FailingTts;
    #[async_trait]
    impl Tts for FailingTts {
        async fn synthesise(&self, _text: &str) -> Result<SpeechChunk, VoiceError> {
            Err(VoiceError::TtsFailed("sidecar down".into()))
        }
    }

    /// An enabled manager (the default is disabled) — the gate is open for the
    /// allowlist tests that assert speech happens.
    fn enabled_vm() -> Arc<VoiceManager> {
        let vm = Arc::new(VoiceManager::new());
        vm.set_enabled(true);
        vm
    }

    fn run_complete() -> Event {
        Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: true })
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
        let phrases = Arc::new(Mutex::new(Vec::<String>::new()));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let mut proj = VoiceProjection::new(
            Arc::new(CapturingTts { phrases: Arc::clone(&phrases) }),
            enabled_vm(),
        );

        for ev in [
            Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: true }),
            Event::Run(RunEvent::Finished { run_id: "r1".into(), ok: false }),
            Event::Run(RunEvent::Transmission(json!({ "tool": "Bash" }))),
        ] {
            proj.handle(&ctx.publish(StreamId::Run("r1".into()), ev)).await.unwrap();
        }
        let spoken = phrases.lock().unwrap();
        assert_eq!(
            spoken.as_slice(),
            ["Run complete.", "Run stopped.", "Waiting for your approval."],
            "allowlisted events must produce the exact phrases in order"
        );
    }

    #[tokio::test]
    async fn silent_on_non_allowlisted_events() {
        let calls = Arc::new(AtomicU32::new(0));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let mut proj =
            VoiceProjection::new(Arc::new(CountingTts { calls: Arc::clone(&calls) }), enabled_vm());

        for ev in [
            Event::Run(RunEvent::WorkflowStep(json!({}))),
            Event::Run(RunEvent::WorkflowDone(json!({}))),
        ] {
            proj.handle(&ctx.publish(StreamId::Run("r1".into()), ev)).await.unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0, "non-allowlisted run events stay silent (SC-004)");
    }

    // T020 / FR-015 + EC-006 — the toggle gate: voice off ⇒ silent on an event
    // that would otherwise be spoken.
    #[tokio::test]
    async fn disabled_voice_stays_silent_on_allowlisted_event() {
        let calls = Arc::new(AtomicU32::new(0));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        // Default manager is *disabled* — do not enable it.
        let vm = Arc::new(VoiceManager::new());
        let mut proj =
            VoiceProjection::new(Arc::new(CountingTts { calls: Arc::clone(&calls) }), vm);

        proj.handle(&ctx.publish(StreamId::Run("r1".into()), run_complete())).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0, "voice off ⇒ no TTS even for an allowlisted event");
    }

    // EC-006 — toggling off mid-stream halts: an allowlisted event after the
    // toggle flips off produces no speech.
    #[tokio::test]
    async fn toggling_off_mid_stream_halts_speech() {
        let calls = Arc::new(AtomicU32::new(0));
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let vm = enabled_vm();
        let mut proj =
            VoiceProjection::new(Arc::new(CountingTts { calls: Arc::clone(&calls) }), Arc::clone(&vm));

        proj.handle(&ctx.publish(StreamId::Run("r1".into()), run_complete())).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "enabled ⇒ spoken");
        vm.set_enabled(false); // toggle off mid-stream
        proj.handle(&ctx.publish(StreamId::Run("r1".into()), run_complete())).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "after toggle-off, no further speech (EC-006)");
    }

    // T019 / FR-014 — a TTS-down failure surfaces a typed error via VoiceStatus,
    // without panicking the handler.
    #[tokio::test]
    async fn tts_failure_surfaces_via_voice_status() {
        let bus = Arc::new(Bus::new(16));
        let ctx = ctx(&bus);
        let vm = enabled_vm();
        let mut proj = VoiceProjection::new(Arc::new(FailingTts), Arc::clone(&vm));

        // Handler returns Ok (no panic) even though synthesis failed.
        proj.handle(&ctx.publish(StreamId::Run("r1".into()), run_complete())).await.unwrap();
        assert_eq!(
            vm.status().last_error.as_deref(),
            Some("text-to-speech failed: sidecar down"),
            "a TTS-down error is surfaced via VoiceStatus (FR-014)"
        );
    }
}
