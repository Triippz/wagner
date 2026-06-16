//! T019 — integration test: the CLI driver spawns a real child process,
//! pumps its stdout through a mapper, and surfaces CliSignals.
//!
//! We use `cat <fixture>` as a deterministic stand-in for a CLI: it emits the
//! recorded stream-json lines, exactly as `claude`/`codex` would, without
//! burning a subscription.

use wagner_edge_host::cli::Driver;
use wagner_edge_host::events::{map_claude_line, map_codex_line, CliSignal};

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[tokio::test]
async fn driver_pumps_claude_fixture_into_signals() {
    let fixture = fixtures_dir().join("claude-sample.jsonl");
    let mut driver = Driver::spawn(
        "cat",
        &[fixture.to_string_lossy().to_string()],
        &fixtures_dir(),
        map_claude_line,
    )
    .expect("spawn cat");

    let ok = driver.wait().await.expect("child waits");
    assert!(ok, "cat should exit 0");

    let signals = driver.collect_remaining().await;
    assert_eq!(signals.first(), Some(&CliSignal::Spawned));
    assert!(
        signals.iter().any(|s| matches!(
            s,
            CliSignal::Completed {
                cost_usd: Some(_),
                ..
            }
        )),
        "claude fixture must yield a Completed signal carrying cost"
    );
}

#[tokio::test]
async fn driver_pumps_codex_fixture_into_signals() {
    let fixture = fixtures_dir().join("codex-sample.jsonl");
    let mut driver = Driver::spawn(
        "cat",
        &[fixture.to_string_lossy().to_string()],
        &fixtures_dir(),
        map_codex_line,
    )
    .expect("spawn cat");

    driver.wait().await.expect("child waits");
    let signals = driver.collect_remaining().await;
    assert_eq!(signals.first(), Some(&CliSignal::Spawned));
    assert!(signals
        .iter()
        .any(|s| matches!(s, CliSignal::Completed { .. })));
}

#[tokio::test]
async fn spawning_a_missing_program_errors_cleanly() {
    let res = Driver::spawn(
        "this-binary-does-not-exist-12345",
        &[],
        &fixtures_dir(),
        map_claude_line,
    );
    assert!(res.is_err(), "missing program must return a Spawn error");
}
