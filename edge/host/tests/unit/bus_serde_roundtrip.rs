//! T005 [US1] — serde round-trip for a representative `Event`/`Command` of every
//! v1 namespace (Run/Goal/Vault/Voice/Ui) + `Ext`, and for the `Envelope` that
//! wraps them. `deserialize(serialize(x)) == x`, byte-stable. Covers SC-001, AS-1.

use wagner_edge_host::bus::{
    Command, Envelope, Event, EventId, GoalCommand, GoalEvent, NodeId, ParticipantId,
    ParticipantKind, RunCommand, RunEvent, Scope, StreamId, Timestamp, UiCommand, UiEvent,
    VaultCommand, VaultEvent, VoiceCommand, VoiceEvent,
};

fn ulid() -> ulid::Ulid {
    "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()
}

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = serde_json::to_vec(value).expect("serialize");
    let back: T = serde_json::from_slice(&bytes).expect("deserialize");
    assert_eq!(value, &back, "round-trip must be identity");
    assert_eq!(serde_json::to_vec(&back).unwrap(), bytes, "re-serialize must be byte-stable");
}

fn one_event_per_namespace() -> Vec<Event> {
    vec![
        Event::Run(RunEvent::Finished { run_id: "01J".into(), ok: true }),
        Event::Goal(GoalEvent::Added { goal_id: "g1".into(), title: "ship".into() }),
        Event::Vault(VaultEvent::NoteUpdated { path: "n.md".into(), rev: 3 }),
        Event::Voice(VoiceEvent::UtteranceTranscribed { text: "hello".into() }),
        Event::Ui(UiEvent::SurfaceFocused { surface: "chat".into() }),
        Event::Ext {
            ns: "slack".into(),
            name: "message".into(),
            version: 1,
            payload: serde_json::json!({ "channel": "C1" }),
        },
    ]
}

fn one_command_per_namespace() -> Vec<Command> {
    vec![
        Command::Run(RunCommand::Start { goal: "build".into() }),
        Command::Goal(GoalCommand::Add { title: "ship".into() }),
        Command::Vault(VaultCommand::UpdateNote { path: "n.md".into(), body: "x".into() }),
        Command::Voice(VoiceCommand::Speak { text: "hi".into() }),
        Command::Ui(UiCommand::FocusSurface { surface: "chat".into() }),
        Command::Ext {
            ns: "slack".into(),
            name: "post_message".into(),
            version: 1,
            payload: serde_json::json!({ "channel": "C1", "text": "hi" }),
        },
    ]
}

fn envelope_around(payload: Event) -> Envelope {
    Envelope::new(
        EventId(ulid()),
        Timestamp("2026-06-19T00:00:00Z".into()),
        ParticipantId {
            node: NodeId("z32-node-id".into()),
            kind: ParticipantKind::System,
            name: "host".into(),
            instance: ulid(),
        },
        StreamId::Run("01J0RUN".into()),
        7,
        Scope { user: "u".into(), workspace: "w".into() },
        payload,
    )
}

#[test]
fn every_event_namespace_round_trips() {
    for event in one_event_per_namespace() {
        round_trip(&event);
    }
}

#[test]
fn every_command_namespace_round_trips() {
    for command in one_command_per_namespace() {
        round_trip(&command);
    }
}

#[test]
fn envelope_round_trips_carrying_each_namespace() {
    for event in one_event_per_namespace() {
        round_trip(&envelope_around(event));
    }
}

// 011 P2 — the variants that carry the real UI payloads (`Snapshot`/`Activity`/
// `DownloadProgress`) are schema-opaque but MUST still serde-round-trip exactly,
// or the `wagner://run|event|voice-download` byte-compat contract breaks. This
// guards against a serde-attribute change to Run/WagnerEvent/ModelProgress that
// the opaque schema would otherwise hide.
#[test]
fn opaque_payload_variants_round_trip() {
    use wagner_edge_host::events::{Activity, District, Faction, OperativeState, WagnerEvent};
    use wagner_edge_host::state::Run;
    use wagner_edge_host::voice::{ModelProgress, ModelState};

    let run = Run::new("r1".into(), "ship it".into(), vec!["d.md".into()], "2026-06-19T00:00:00Z".into());
    round_trip(&Event::Run(RunEvent::Snapshot(Box::new(run.clone()))));
    round_trip(&envelope_around(Event::Run(RunEvent::Snapshot(Box::new(run)))));

    let activity = WagnerEvent {
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
    };
    round_trip(&Event::Run(RunEvent::Activity(Box::new(activity))));

    let progress = ModelProgress { model: "stt".into(), state: ModelState::Downloading, received: 5, total: 10 };
    round_trip(&Event::Voice(VoiceEvent::DownloadProgress(Box::new(progress))));
}
