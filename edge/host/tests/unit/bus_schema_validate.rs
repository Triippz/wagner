//! T006 [US1] — schema accept/reject at the boundary + committed-catalog drift
//! guard + schema-version id + per-type stability tier.
//! T009 [US1] — `PluginManifest` expresses all fields; a zero-capability manifest
//! is valid.
//! Covers SC-002, SC-003, SC-008, AS-2, AS-4, EC-003, EC-004, FR-012, FR-014,
//! FR-015, FR-016.

use std::collections::HashMap;
use wagner_edge_host::bus::{
    export_schemas, Capability, Command, Contract, Envelope, Event, EventId, Namespace, NodeId,
    ParticipantId, ParticipantKind, PluginManifest, RunCommand, Scope, SchemaRef, StabilityTier,
    StreamId, Timestamp, UiEvent, VaultEvent,
};
use wagner_edge_host::schema::validate;

fn schemas() -> HashMap<String, String> {
    export_schemas()
        .into_iter()
        .map(|(name, value)| (name, serde_json::to_string(&value).unwrap()))
        .collect()
}

fn ulid() -> ulid::Ulid {
    "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()
}

fn sample_envelope(payload: Event) -> Envelope {
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
        0,
        Scope { user: "u".into(), workspace: "w".into() },
        payload,
    )
}

#[test]
fn each_core_payload_validates_against_its_exported_schema() {
    let s = schemas();
    let event = Event::Vault(VaultEvent::NoteUpdated { path: "n.md".into(), rev: 1 });
    validate(&s["event"], &serde_json::to_value(&event).unwrap()).expect("event validates");

    let command = Command::Run(RunCommand::Start { goal: "x".into() });
    validate(&s["command"], &serde_json::to_value(&command).unwrap()).expect("command validates");

    let envelope = sample_envelope(event);
    validate(&s["envelope"], &serde_json::to_value(&envelope).unwrap()).expect("envelope validates");
}

#[test]
fn extra_or_wrong_typed_field_is_rejected_at_boundary() {
    let s = schemas();
    let envelope = sample_envelope(Event::Ui(UiEvent::SurfaceFocused { surface: "chat".into() }));

    let mut extra = serde_json::to_value(&envelope).unwrap();
    extra.as_object_mut().unwrap().insert("rogue".into(), serde_json::json!(1));
    assert!(
        validate(&s["envelope"], &extra).is_err(),
        "SC-003/EC-003: an extra field must be rejected (additionalProperties:false)"
    );

    let mut wrong = serde_json::to_value(&envelope).unwrap();
    wrong["seq"] = serde_json::json!("not-a-number");
    assert!(validate(&s["envelope"], &wrong).is_err(), "a wrong-typed field must be rejected");
}

#[test]
fn persisted_envelope_carries_schema_version_and_types_carry_tier() {
    let envelope = sample_envelope(Event::Vault(VaultEvent::NoteUpdated { path: "n".into(), rev: 1 }));
    assert_eq!(envelope.schema, "envelope.v1", "FR-016: persisted payload carries its schema-version id");

    // SC-008: every core contract type carries a stability tier.
    assert_eq!(Envelope::TIER, StabilityTier::Stable);
    assert_eq!(Event::TIER, StabilityTier::Stable);
    assert_eq!(Command::TIER, StabilityTier::Stable);
    assert_eq!(PluginManifest::TIER, StabilityTier::Stable);
}

#[test]
fn committed_catalog_matches_fresh_export_or_regenerates() {
    // Drift guard (SC-002): the committed schemas under schemas/bus/ must equal a
    // fresh schemars export. Regeneration is gated behind UPDATE_SCHEMAS=1.
    let update = std::env::var("UPDATE_SCHEMAS").is_ok();
    for (name, value) in export_schemas() {
        let path = format!("schemas/bus/{name}.json");
        if update {
            let pretty = serde_json::to_string_pretty(&value).unwrap() + "\n";
            std::fs::write(&path, pretty).unwrap();
            continue;
        }
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("committed schema missing: {path} — run UPDATE_SCHEMAS=1"));
        let committed: serde_json::Value = serde_json::from_str(&committed).unwrap();
        assert_eq!(committed, value, "{name}.json drifted from the Rust type; regenerate with UPDATE_SCHEMAS=1");
    }
}

#[test]
fn manifest_expresses_all_fields_and_zero_capability_is_valid() {
    let s = schemas();

    let manifest = PluginManifest {
        participants_provided: vec!["my-agent".into()],
        emits: vec![Namespace::Vault],
        subscribes: vec![Namespace::Run, Namespace::Goal],
        registered_schemas: vec![SchemaRef { name: "ext.slack.message".into(), version: 1 }],
        capabilities: vec![Capability::VaultRead, Capability::Network],
        stability: StabilityTier::Experimental,
    };
    validate(&s["plugin_manifest"], &serde_json::to_value(&manifest).unwrap())
        .expect("a full manifest validates (FR-012/FR-014)");

    // EC-004: a pure subscriber that requests no capabilities is valid.
    let pure_subscriber = PluginManifest {
        participants_provided: vec!["ui-gateway".into()],
        emits: vec![],
        subscribes: vec![Namespace::Ui],
        registered_schemas: vec![],
        capabilities: vec![],
        stability: StabilityTier::Stable,
    };
    validate(&s["plugin_manifest"], &serde_json::to_value(&pure_subscriber).unwrap())
        .expect("a zero-capability manifest must be valid");
}

#[test]
fn ext_event_validates_against_registered_schema_with_zero_core_edits() {
    // T020 [US2] — SC-006, AS-1, AS-2, EC-005.
    let ok = Event::Ext {
        ns: "slack".into(),
        name: "message".into(),
        version: 1,
        payload: serde_json::json!({ "channel": "C1", "text": "hello" }),
    };
    ok.validate_ext().expect("a registered Ext payload validates (SC-006)");

    // EC-005: an Ext payload with an unexpected field is rejected by the
    // registered schema's `additionalProperties:false`.
    let bad = Event::Ext {
        ns: "slack".into(),
        name: "message".into(),
        version: 1,
        payload: serde_json::json!({ "channel": "C1", "text": "hi", "rogue": true }),
    };
    assert!(bad.validate_ext().is_err(), "EC-005: an unexpected field must be rejected");

    // An unregistered extension triple resolves to no schema.
    assert!(Event::ext_schema("slack", "message", 2).is_none());

    // SC-006: extending via `Ext` added ZERO new core namespaces — the core
    // `Event` enum still exposes exactly the six v1 namespaces.
    let (_, event_schema) = export_schemas().into_iter().find(|(n, _)| n == "event").unwrap();
    let namespaces: Vec<&str> = event_schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["properties"]["type"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(
        namespaces,
        ["run", "goal", "vault", "voice", "ui", "ext"],
        "the Ext seam must not add a core namespace variant"
    );
}
