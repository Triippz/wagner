//! UiGateway (spec 011 P2) — bridges the typed event bus back to the legacy
//! `wagner://*` Tauri channels so the React surface is **unchanged** during the
//! migration (strangler-fig: lowest-risk first step).
//!
//! The emit side now publishes typed [`Event`]s to the [`Bus`]; a single
//! background task subscribes and re-emits each event to its legacy Tauri channel
//! via [`project`] — byte-identically, because each variant carries the exact
//! payload the old `app.emit(...)` sent. `project` is pure, so the byte-compat
//! contract is unit-tested without a live Tauri app.

use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use ulid::Ulid;
use wagner_edge_host::bus::{
    Bus, Envelope, Event, EventId, NodeId, ParticipantId, ParticipantKind, RecvError, RunEvent,
    Scope, StreamId, Subscription, Timestamp, UiEvent, VoiceEvent,
};

/// Identity stamped on envelopes the shell publishes on behalf of the engine/UI.
/// One logical participant per process → a stable `instance`.
fn ui_origin() -> ParticipantId {
    ParticipantId {
        node: NodeId("local".into()),
        kind: ParticipantKind::Ui,
        name: "ui-gateway".into(),
        instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("valid seed ulid"),
    }
}

/// Wrap a typed event in an envelope (the bus stamps the authoritative `seq`).
fn envelope(stream: StreamId, payload: Event) -> Envelope {
    Envelope::new(
        EventId(Ulid::new()),
        Timestamp(chrono::Utc::now().to_rfc3339()),
        ui_origin(),
        stream,
        0,
        Scope { user: "local".into(), workspace: "local".into() },
        payload,
    )
}

/// The publish-side handle held in Tauri state and captured by the emit closures.
#[derive(Clone)]
pub struct UiGateway {
    bus: Arc<Bus>,
}

impl UiGateway {
    pub fn new(bus: Arc<Bus>) -> Self {
        Self { bus }
    }

    /// Publish a typed event on a run-scoped stream.
    pub fn publish_run(&self, run_id: &str, payload: Event) {
        self.bus.publish(envelope(StreamId::Run(run_id.to_string()), payload));
    }

    /// Publish a workspace-scoped event (no run context, e.g. voice downloads).
    pub fn publish_workspace(&self, workspace: &str, payload: Event) {
        self.bus.publish(envelope(StreamId::Workspace(workspace.to_string()), payload));
    }
}

/// Map a typed bus event to its legacy `(channel, payload)`. Returns `None` for
/// events with no legacy UI channel (e.g. the contract seed variants). **The
/// byte-compat seam** — each arm reproduces exactly what the pre-bus
/// `app.emit(...)` call site sent.
pub fn project(event: &Event) -> Option<(&'static str, serde_json::Value)> {
    use serde_json::to_value;
    let out = match event {
        Event::Run(RunEvent::Activity(ev)) => ("wagner://event", to_value(ev).ok()?),
        Event::Run(RunEvent::Snapshot(run)) => ("wagner://run", to_value(run).ok()?),
        Event::Run(RunEvent::Transmission(v)) => ("wagner://transmission", v.clone()),
        Event::Run(RunEvent::WorkflowStep(v)) => ("wagner://workflow", v.clone()),
        Event::Run(RunEvent::WorkflowDone(v)) => ("wagner://workflow-done", v.clone()),
        Event::Ui(UiEvent::Panel { operative_id, spec }) => (
            "wagner://panel",
            serde_json::json!({ "operative_id": operative_id, "spec": spec }),
        ),
        Event::Voice(VoiceEvent::DownloadProgress(p)) => ("wagner://voice-download", to_value(p).ok()?),
        _ => return None,
    };
    Some(out)
}

/// Spawn the gateway: subscribe to the whole bus and re-emit each event to its
/// legacy Tauri channel. Survives a slow tick (`Lagged`); stops when the bus
/// closes (app exit).
pub fn spawn(bus: Arc<Bus>, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut sub = bus.subscribe(Subscription { topic: "*".into(), filter: None });
        loop {
            match sub.recv().await {
                Ok(env) => {
                    if let Some((channel, payload)) = project(&env.payload) {
                        let _ = app.emit(channel, payload);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wagner_edge_host::events::{Activity, District, Faction, OperativeState, WagnerEvent};
    use wagner_edge_host::state::Run;
    use wagner_edge_host::voice::{ModelProgress, ModelState};

    fn sample_event() -> WagnerEvent {
        WagnerEvent {
            schema: "wagner-event.v1".into(),
            event_id: "01J0000000000000000000000A".into(),
            run_id: "01J0000000000000000000000B".into(),
            operative_id: "cipher".into(),
            operative_name: "Cipher".into(),
            faction: Faction::Architects,
            activity: Activity::Edit,
            district: District::Stacks,
            state: OperativeState::Working,
            message: Some("editing".into()),
            handoff_target_operative_id: None,
            ts: "2026-06-19T00:00:00Z".into(),
        }
    }

    // P2 golden: bus event → project() → (channel, payload) is byte-identical to
    // what the pre-bus `app.emit(...)` site sent, for each of the 7 channels.

    #[test]
    fn activity_projects_to_legacy_event_channel() {
        let ev = sample_event();
        let (chan, payload) = project(&Event::Run(RunEvent::Activity(Box::new(ev.clone())))).unwrap();
        assert_eq!(chan, "wagner://event");
        assert_eq!(payload, serde_json::to_value(&ev).unwrap(), "byte-identical to old app.emit");
    }

    #[test]
    fn snapshot_projects_to_legacy_run_channel() {
        let run = Run::new("r1".into(), "ship it".into(), vec![], "2026-06-19T00:00:00Z".into());
        let (chan, payload) = project(&Event::Run(RunEvent::Snapshot(Box::new(run.clone())))).unwrap();
        assert_eq!(chan, "wagner://run");
        assert_eq!(payload, serde_json::to_value(&run).unwrap());
    }

    #[test]
    fn panel_projects_to_legacy_panel_channel() {
        let spec = serde_json::json!({ "kind": "markdown", "body": "hi" });
        let (chan, payload) =
            project(&Event::Ui(UiEvent::Panel { operative_id: "cipher".into(), spec: spec.clone() })).unwrap();
        assert_eq!(chan, "wagner://panel");
        assert_eq!(payload, serde_json::json!({ "operative_id": "cipher", "spec": spec }));
    }

    #[test]
    fn transmission_workflow_pass_through_verbatim() {
        let tj = serde_json::json!({ "schema": "transmission.v1", "id": "x", "kind": "permission" });
        let (chan, payload) = project(&Event::Run(RunEvent::Transmission(tj.clone()))).unwrap();
        assert_eq!(chan, "wagner://transmission");
        assert_eq!(payload, tj);

        let step = serde_json::json!({ "run_id": "r", "node_id": "n", "kind": "plan" });
        let (chan, payload) = project(&Event::Run(RunEvent::WorkflowStep(step.clone()))).unwrap();
        assert_eq!(chan, "wagner://workflow");
        assert_eq!(payload, step);

        let done = serde_json::json!({ "run_id": "r", "end": "Completed", "cost": 0.0 });
        let (chan, payload) = project(&Event::Run(RunEvent::WorkflowDone(done.clone()))).unwrap();
        assert_eq!(chan, "wagner://workflow-done");
        assert_eq!(payload, done);
    }

    #[test]
    fn voice_download_projects_to_legacy_channel() {
        let p = ModelProgress { model: "stt".into(), state: ModelState::Downloading, received: 5, total: 10 };
        let (chan, payload) = project(&Event::Voice(VoiceEvent::DownloadProgress(Box::new(p.clone())))).unwrap();
        assert_eq!(chan, "wagner://voice-download");
        assert_eq!(payload, serde_json::to_value(&p).unwrap());
    }

    #[test]
    fn seed_and_non_ui_events_have_no_legacy_channel() {
        assert!(project(&Event::Run(RunEvent::Finished { run_id: "r".into(), ok: true })).is_none());
        assert!(project(&Event::Ui(UiEvent::SurfaceFocused { surface: "console".into() })).is_none());
    }
}
