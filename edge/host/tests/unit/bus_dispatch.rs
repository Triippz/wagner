//! 011 P3 — command intake (`Bus::dispatch`). The single validated, authorized
//! path a command takes before reaching the bus: a valid command is authorized,
//! stamped, and enqueued; an unauthorized one is denied; schema-invalid JSON is
//! rejected at the boundary before it can become a typed `Command`.

use wagner_edge_host::bus::{
    AllowAll, Bus, Command, CommandAuthorizer, DispatchError, RunCommand,
};

/// A policy that denies everything — exercises the Article IX deny path.
struct DenyAll;
impl CommandAuthorizer for DenyAll {
    fn authorize(&self, _command: &Command) -> Result<(), String> {
        Err("not allowed".into())
    }
}

#[test]
fn valid_command_is_authorized_stamped_and_enqueued() {
    let bus = Bus::new(8);
    let mut rx = bus.take_commands().expect("receiver available");

    let accepted = bus
        .dispatch(Command::Run(RunCommand::Abort { run_id: Some("r1".into()) }), &AllowAll)
        .expect("valid command accepted");

    let got = rx.try_recv().expect("command enqueued");
    assert_eq!(got.id, accepted.id, "the enqueued envelope carries the accepted id");
    assert_eq!(got.command, Command::Run(RunCommand::Abort { run_id: Some("r1".into()) }));
}

#[test]
fn denied_command_is_rejected_and_not_enqueued() {
    let bus = Bus::new(8);
    let mut rx = bus.take_commands().expect("receiver available");

    let err = bus
        .dispatch(Command::Run(RunCommand::Start { goal: "x".into() }), &DenyAll)
        .expect_err("DenyAll must reject");
    assert!(matches!(err, DispatchError::Denied(_)));
    assert!(rx.try_recv().is_err(), "a denied command never reaches the intake");
}

#[test]
fn schema_invalid_json_is_rejected_at_the_boundary() {
    let bus = Bus::new(8);
    let _rx = bus.take_commands();

    // Not a valid Command shape (no adjacently-tagged {type,data}).
    let bad = serde_json::json!({ "nonsense": true });
    let err = bus.dispatch_json(&bad, &AllowAll).expect_err("schema-invalid rejected");
    assert!(matches!(err, DispatchError::Invalid(_)));
}

#[test]
fn valid_json_command_dispatches() {
    let bus = Bus::new(8);
    let mut rx = bus.take_commands().expect("receiver available");

    let raw = serde_json::to_value(Command::Run(RunCommand::Steer {
        run_id: "r1".into(),
        text: "focus tests".into(),
    }))
    .unwrap();
    bus.dispatch_json(&raw, &AllowAll).expect("valid json command accepted");

    let got = rx.try_recv().expect("command enqueued");
    assert_eq!(got.command, Command::Run(RunCommand::Steer { run_id: "r1".into(), text: "focus tests".into() }));
}

#[test]
fn backpressure_when_intake_is_full_and_undrained() {
    // capacity 1, receiver taken but never drained → second dispatch backs up.
    let bus = Bus::new(1);
    let _rx = bus.take_commands().expect("receiver available");
    bus.dispatch(Command::Run(RunCommand::Abort { run_id: None }), &AllowAll).expect("first fits");
    let err = bus
        .dispatch(Command::Run(RunCommand::Abort { run_id: None }), &AllowAll)
        .expect_err("intake full");
    assert_eq!(err, DispatchError::Backpressure);
}

// ── T011 — reject: unauthorized or schema-invalid run-control command ──────────

/// An unauthorized abort is denied at intake and never reaches a live run.
#[test]
fn unauthorized_abort_is_denied_at_intake() {
    let bus = Bus::new(8);
    let mut rx = bus.take_commands().expect("receiver");

    let err = bus
        .dispatch(Command::Run(RunCommand::Abort { run_id: Some("r1".into()) }), &DenyAll)
        .expect_err("DenyAll must reject abort");
    assert!(matches!(err, DispatchError::Denied(_)), "abort denied by policy: {err:?}");
    assert!(rx.try_recv().is_err(), "denied abort never reaches the intake");
}

/// An unauthorized steer is denied at intake — no live run is affected.
#[test]
fn unauthorized_steer_is_denied_at_intake() {
    let bus = Bus::new(8);
    let mut rx = bus.take_commands().expect("receiver");

    let err = bus
        .dispatch(
            Command::Run(RunCommand::Steer { run_id: "r1".into(), text: "pivot".into() }),
            &DenyAll,
        )
        .expect_err("DenyAll must reject steer");
    assert!(matches!(err, DispatchError::Denied(_)), "steer denied by policy: {err:?}");
    assert!(rx.try_recv().is_err(), "denied steer never reaches the intake");
}

/// A schema-invalid JSON run-control payload is rejected before it can reach
/// any live run, regardless of the authorizer.
#[test]
fn schema_invalid_run_control_rejected_at_boundary() {
    let bus = Bus::new(8);
    let _rx = bus.take_commands();

    // Valid outer envelope type but wrong inner shape (no run_id, not a valid Command).
    let bad = serde_json::json!({ "type": "run", "data": { "action": "explode" } });
    let err = bus.dispatch_json(&bad, &AllowAll).expect_err("schema-invalid rejected");
    assert!(matches!(err, DispatchError::Invalid(_)), "invalid payload rejected: {err:?}");
}
