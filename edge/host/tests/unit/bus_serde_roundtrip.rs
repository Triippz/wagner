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
