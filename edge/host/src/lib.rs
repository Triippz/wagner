//! Wagner edge host — engine core, ported from `apps/wagner/src-tauri/src`
//! (T000a, 2026-06-16). This is the **headless** library: the goal loop, the
//! permission/guardrail gate, the append-only run-state store, event mapping,
//! and the embedded memory store. The Tauri shell (`run()`, `ipc::commands`,
//! the tray) is deferred to wedge-002 US1 (T019/T021) and is NOT part of this
//! crate yet — nothing here links Tauri, so the engine builds and tests
//! headlessly (Article VI: the run completes with no shell, no hub, no remote).

pub mod cli;
pub mod events;
pub mod memory;
pub mod orchestrator;
pub mod permission_server;
pub mod schema;
pub mod state;
pub mod transmissions;
pub mod vault;

// wedge-002 surfaces (scaffolds, T002) — implemented test-first in later phases.
pub mod remote;
pub mod tray;

#[cfg(test)]
mod foundation_tests {
    use crate::events::{
        activity_to_district, activity_to_state, Activity, District, Faction, OperativeState,
        WagnerEvent,
    };
    use crate::schema::{self, CONSTRUCT_EVENT_SCHEMA};

    fn sample_event() -> WagnerEvent {
        WagnerEvent {
            schema: WagnerEvent::SCHEMA.to_string(),
            event_id: "01J0000000000000000000000A".to_string(),
            run_id: "01J0000000000000000000000B".to_string(),
            operative_id: "cipher".to_string(),
            operative_name: "Cipher".to_string(),
            faction: Faction::Architects,
            activity: Activity::Edit,
            district: District::Stacks,
            state: OperativeState::Working,
            message: Some("editing lib.rs".to_string()),
            handoff_target_operative_id: None,
            ts: "2026-06-13T00:00:00Z".to_string(),
        }
    }

    // T011: a WagnerEvent serializes to schema-valid JSON.
    #[test]
    fn wagner_event_validates_against_schema() {
        let json = schema::validate_serialized(CONSTRUCT_EVENT_SCHEMA, &sample_event())
            .expect("sample event must validate against wagner-event.schema.json");
        assert_eq!(json["schema"], "wagner-event.v1");
    }

    // T011: a malformed event (wrong schema const) is rejected.
    #[test]
    fn wagner_event_rejects_bad_schema_const() {
        let mut ev = sample_event();
        ev.schema = "wrong.v1".to_string();
        assert!(schema::validate_serialized(CONSTRUCT_EVENT_SCHEMA, &ev).is_err());
    }

    // T015: activity→district mapping (R-EVENT) is exhaustive and correct.
    #[test]
    fn activity_district_mapping_is_correct() {
        assert_eq!(activity_to_district(Activity::Read), District::Stacks);
        assert_eq!(activity_to_district(Activity::Edit), District::Stacks);
        assert_eq!(activity_to_district(Activity::Test), District::Forge);
        assert_eq!(activity_to_district(Activity::Build), District::Forge);
        assert_eq!(activity_to_district(Activity::Lint), District::Forge);
        assert_eq!(activity_to_district(Activity::Shell), District::Forge);
        assert_eq!(activity_to_district(Activity::Review), District::Mirror);
        assert_eq!(activity_to_district(Activity::Diff), District::Mirror);
        assert_eq!(activity_to_district(Activity::Judge), District::Mirror);
        assert_eq!(activity_to_district(Activity::Plan), District::Oracle);
        assert_eq!(activity_to_district(Activity::Decompose), District::Oracle);
        assert_eq!(activity_to_district(Activity::Think), District::Oracle);
        assert_eq!(activity_to_district(Activity::AwaitPermission), District::Gate);
        assert_eq!(activity_to_district(Activity::AwaitQuestion), District::Gate);
    }

    // T015: blocked-state derivation.
    #[test]
    fn await_activities_are_blocked() {
        assert_eq!(
            activity_to_state(Activity::AwaitPermission),
            OperativeState::Blocked
        );
        assert_eq!(activity_to_state(Activity::Edit), OperativeState::Working);
        assert_eq!(activity_to_state(Activity::Plan), OperativeState::Thinking);
    }

    // Article VII: all four embedded schemas are themselves valid JSON Schema.
    #[test]
    fn all_schemas_are_well_formed() {
        for src in [
            CONSTRUCT_EVENT_SCHEMA,
            schema::RUN_STATE_SCHEMA,
            schema::ORACLE_PLAN_SCHEMA,
            schema::TRANSMISSION_SCHEMA,
        ] {
            let v: serde_json::Value = serde_json::from_str(src).expect("schema is valid JSON");
            jsonschema::JSONSchema::compile(&v).expect("schema compiles as JSON Schema");
        }
    }
}

#[cfg(test)]
mod state_tests {
    use crate::state::{self, Run, RunStatus, WriteOutcome};

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("wagner-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn sample_run() -> Run {
        Run::new(
            "01J0000000000000000000000R".to_string(),
            "build the thing".to_string(),
            vec!["docs/x.md".to_string()],
            "2026-06-13T00:00:00Z".to_string(),
        )
    }

    // T012: a run round-trips through atomic save/load and validates against the schema.
    #[test]
    fn run_round_trips_through_store() {
        let root = temp_root("roundtrip");
        let run = sample_run();
        let outcome = state::save(&root, &run).expect("save must succeed and validate");
        assert_eq!(outcome, WriteOutcome::Created);

        let loaded = state::load(&root, &run.run_id).expect("load must succeed");
        assert_eq!(loaded, run);
        assert_eq!(loaded.status, RunStatus::Drafted);
        let _ = std::fs::remove_dir_all(&root);
    }

    // T012 / Article V: a second save reports Updated, not Created.
    #[test]
    fn second_save_reports_updated() {
        let root = temp_root("updated");
        let mut run = sample_run();
        assert_eq!(state::save(&root, &run).unwrap(), WriteOutcome::Created);
        run.status = RunStatus::Running;
        run.iteration = 1;
        assert_eq!(state::save(&root, &run).unwrap(), WriteOutcome::Updated);
        assert_eq!(state::load(&root, &run.run_id).unwrap().iteration, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    // T012: an empty goal is rejected by the schema (minLength 1) before hitting disk.
    #[test]
    fn empty_goal_rejected_by_schema() {
        let root = temp_root("emptygoal");
        let mut run = sample_run();
        run.goal = String::new();
        assert!(state::save(&root, &run).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    // T067 / Gate VI: run-state serialization is byte-identical for identical input.
    #[test]
    fn run_state_serialization_is_deterministic() {
        let run = sample_run();
        let a = serde_json::to_vec_pretty(&run).unwrap();
        let b = serde_json::to_vec_pretty(&run).unwrap();
        assert_eq!(a, b, "identical Run must serialize to identical bytes");
    }

    // Article V: no temp file is left behind after a successful write.
    #[test]
    fn no_temp_file_left_behind() {
        let root = temp_root("notmp");
        let run = sample_run();
        state::save(&root, &run).unwrap();
        let tmp = state::run_state_path(&root, &run.run_id).with_extension("json.tmp");
        assert!(!tmp.exists(), "temp file must be renamed away");
        let _ = std::fs::remove_dir_all(&root);
    }
}
