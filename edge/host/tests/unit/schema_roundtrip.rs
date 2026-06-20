//! T022 [P] [US2] — Schema round-trip test.
//!
//! `Run`, `WagnerEvent`, and `ModelProgress` each:
//!   (a) validate against their declared JSON Schema (draft 2020-12), and
//!   (b) serde round-trip (`deserialize(serialize(x)) == x`).
//!
//! Covers FR-010, US2-AS1, D-TEST-3.

use wagner_edge_host::events::{Activity, District, Faction, OperativeState, WagnerEvent};
use wagner_edge_host::schema::{validate, SchemaError};
use wagner_edge_host::state::Run;
use wagner_edge_host::voice::{ModelProgress, ModelState};
use schemars::schema_for;

// ── helpers ───────────────────────────────────────────────────────────────────

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = serde_json::to_vec(value).expect("serialize");
    let back: T = serde_json::from_slice(&bytes).expect("deserialize");
    assert_eq!(value, &back, "round-trip must be identity");
}

/// Generate a JSON Schema string from the schemars derive on `T`.
fn schema_str<T: schemars::JsonSchema>() -> String {
    let schema = schema_for!(T);
    serde_json::to_string(&schema).expect("schema serializes")
}

fn validate_against_derived<T: schemars::JsonSchema + serde::Serialize>(
    value: &T,
) -> Result<(), SchemaError> {
    let schema = schema_str::<T>();
    let json = serde_json::to_value(value).expect("serialize for validation");
    validate(&schema, &json)
}

// ── Run ───────────────────────────────────────────────────────────────────────

fn sample_run() -> Run {
    Run::new(
        "01J0RUN0000000000000000001".into(),
        "ship the feature".into(),
        vec!["README.md".into()],
        "2026-06-19T00:00:00Z".into(),
    )
}

#[test]
fn run_validates_against_its_declared_schema() {
    let run = sample_run();
    validate_against_derived(&run).expect("Run must validate against its derived JSON Schema");
}

#[test]
fn run_serde_round_trips() {
    round_trip(&sample_run());
}

// ── WagnerEvent ───────────────────────────────────────────────────────────────

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
        message: Some("editing utils.rs".into()),
        handoff_target_operative_id: None,
        ts: "2026-06-19T00:00:00Z".into(),
    }
}

#[test]
fn wagner_event_validates_against_its_declared_schema() {
    let ev = sample_event();
    validate_against_derived(&ev)
        .expect("WagnerEvent must validate against its derived JSON Schema");
}

#[test]
fn wagner_event_serde_round_trips() {
    round_trip(&sample_event());
}

// ── ModelProgress ─────────────────────────────────────────────────────────────

fn sample_progress() -> ModelProgress {
    ModelProgress {
        model: "stt".into(),
        state: ModelState::Downloading,
        received: 512,
        total: 1024,
    }
}

#[test]
fn model_progress_validates_against_its_declared_schema() {
    let p = sample_progress();
    validate_against_derived(&p)
        .expect("ModelProgress must validate against its derived JSON Schema");
}

#[test]
fn model_progress_serde_round_trips() {
    round_trip(&sample_progress());
}
