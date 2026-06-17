//! CRDT→file flush tests — flush_to_file behaviour.

use std::sync::Mutex;

use loro::LoroDoc;
use uuid::Uuid;
use wagner_edge_host::vault::projector::{FlushResult, ProjectorState, flush_to_file};

fn make_doc_with_content(initial: &str) -> Mutex<LoroDoc> {
    let doc = LoroDoc::new();
    if !initial.is_empty() {
        doc.get_text("content").insert(0, initial).unwrap();
        doc.commit();
    }
    Mutex::new(doc)
}

#[test]
fn test_flush_skipped_when_edit_in_flight() {
    let doc = make_doc_with_content("hello");
    let mut state = ProjectorState::new(Uuid::new_v4(), "hello");

    // Mark edit in flight directly.
    state.edit_in_flight = true;

    let result = flush_to_file(&mut state, &doc).unwrap();
    assert!(
        matches!(result, FlushResult::Skipped),
        "flush must be skipped while edit_in_flight is true"
    );
}

#[test]
fn test_flush_written_when_content_changed() {
    let doc = make_doc_with_content("hello");
    // State hash is for empty string — mismatches the doc content "hello".
    let mut state = ProjectorState::new(Uuid::new_v4(), "");

    let result = flush_to_file(&mut state, &doc).unwrap();
    match result {
        FlushResult::Written(content) => {
            assert_eq!(content, "hello", "flushed content must match CRDT text");
        }
        FlushResult::Skipped => panic!("expected Written, got Skipped"),
    }
}
