//! T044 — Attach first-frame perf harness (SC-001).
//!
//! SC-001's target — remote attach first-frame p50 ≤ 3 s over a simulated
//! residential-NAT↔residential-NAT path with relay available — is measured LIVE
//! (integration-only; needs a real NAT path + relay). This harness exercises the
//! in-memory attach path so the measurement scaffold exists and the local
//! decision cost is bounded well under budget; it is NOT the live SC-001 number.

use std::time::Instant;
use wagner_edge_host::remote::arm::{arm, LocalOperator};
use wagner_edge_host::remote::endpoint::{select_path, LoopbackStream};
use wagner_edge_host::remote::session::{attach, VerifiedRemote};

/// SC-001 budget.
const FIRST_FRAME_BUDGET_MS: u128 = 3_000;

#[test]
fn in_memory_attach_first_frame_is_well_under_budget() {
    let start = Instant::now();

    // Arm → select path → attach → deliver the first frame.
    let (armed, _) = arm(&LocalOperator::from_host("op-1"), "node", "tkt", 0, 60_000);
    let path = select_path(true, true).expect("a path is available");
    let who = VerifiedRemote::new("op-1", "cli-1");
    let _session = attach(&armed, &who, 1).expect("owner attach");
    let mut stream = LoopbackStream::new();
    stream.push(r#"{"channel":"run","payload":{"goal":"x"}}"#);
    let first = stream.drain().into_iter().next().expect("first frame delivered");

    let elapsed = start.elapsed().as_millis();
    assert!(first.contains("goal"));
    assert!(
        elapsed < FIRST_FRAME_BUDGET_MS,
        "in-memory attach path {elapsed}ms must be well under the {FIRST_FRAME_BUDGET_MS}ms SC-001 budget (path={path:?})",
    );
}
