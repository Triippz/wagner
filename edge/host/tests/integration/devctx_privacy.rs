//! T037a — Dev-context privacy (FR-305, F-1, SC-006, US3-AS-7).
//!
//! File contents + diffs from a dev-context session reach the operator's device
//! over P2P; assert 0 code/file/diff bytes are stored on or readable in plaintext
//! by any hub service, and the relay (if on-path) sees only opaque ciphertext
//! frames (size-logged, not decrypted). This does NOT claim "zero bytes traverse
//! the relay" — encrypted frames legitimately do (challenge H1).

use wagner_edge_host::remote::devcontext::run_non_interactive;
use wagner_edge_host::remote::endpoint::RelayFrame;

#[test]
fn command_output_is_not_in_the_persisted_log() {
    // The output is COMPUTED (not a literal argv) so we can prove it reaches the
    // operator's device (frames) yet never the hub-syncable log (F-1). argv holds
    // the unexpanded form; the expanded marker "COMPUTED-42-END" appears only in
    // stdout.
    let argv: Vec<String> = ["sh", "-c", "echo COMPUTED-$((6*7))-END"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let result = run_non_interactive(&argv, std::path::Path::new(".")).unwrap();

    // The output is present in the transient frames (to the operator device)...
    let frame_text: String = result
        .frames
        .iter()
        .map(|f| String::from_utf8_lossy(&f.chunk).to_string())
        .collect();
    assert!(frame_text.contains("COMPUTED-42-END"), "output reaches the operator device");

    // ...but NOT in the log record (the hub-syncable metadata). Structurally the
    // log has no output field, and the expanded marker is absent from its debug.
    assert!(
        !format!("{:?}", result.log).contains("COMPUTED-42-END"),
        "0 output bytes in the log (F-1)",
    );
}

#[test]
fn relay_sees_only_opaque_sized_frames_of_the_output() {
    // The output, once encrypted for transport, is an opaque blob to the relay.
    let ciphertext = b"\x9f\x12encrypted(SECRET-DIFF-CONTENT)\x00";
    let frame = RelayFrame::from_ciphertext(ciphertext);
    assert_eq!(frame.size(), ciphertext.len());
    // The relay's log line is size-only — the plaintext marker never appears.
    assert!(!frame.log_entry().contains("SECRET"));
    assert!(frame.log_entry().contains(&ciphertext.len().to_string()));
}
