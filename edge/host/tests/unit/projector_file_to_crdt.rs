//! File→CRDT projector tests — ingest_file_change behaviour.

use std::sync::Mutex;

use loro::LoroDoc;
use uuid::Uuid;
use wagner_edge_host::vault::projector::{
    sha256_bytes, ProjectorError, ProjectorState, ingest_file_change,
};

fn make_doc_with_content(initial: &str) -> Mutex<LoroDoc> {
    let doc = LoroDoc::new();
    if !initial.is_empty() {
        doc.get_text("content").insert(0, initial).unwrap();
        doc.commit();
    }
    Mutex::new(doc)
}

#[test]
fn test_ingest_applies_diff() {
    let doc = make_doc_with_content("hello");
    let mut state = ProjectorState::new(Uuid::new_v4(), "hello");
    let hash = state.last_projected_hash;

    ingest_file_change(&mut state, &doc, hash, "hello world").unwrap();

    let locked = doc.lock().unwrap();
    assert_eq!(locked.get_text("content").to_string(), "hello world");
}

#[test]
fn test_ingest_stale_hash_rejected() {
    let doc = make_doc_with_content("hello");
    let mut state = ProjectorState::new(Uuid::new_v4(), "hello");

    let wrong_hash = sha256_bytes(b"stale");
    let result = ingest_file_change(&mut state, &doc, wrong_hash, "hello world");

    assert!(
        matches!(result, Err(ProjectorError::StaleHash)),
        "stale hash must be rejected"
    );
}

#[test]
fn test_ingest_sets_edit_in_flight() {
    let doc = make_doc_with_content("");
    let mut state = ProjectorState::new(Uuid::new_v4(), "");
    assert!(!state.edit_in_flight, "initially not in flight");

    let hash = state.last_projected_hash;
    ingest_file_change(&mut state, &doc, hash, "new content").unwrap();

    assert!(state.edit_in_flight, "edit_in_flight must be set after ingest");
}

#[test]
fn test_ingest_multibyte_utf8() {
    // "日本語" is 9 bytes but 3 Unicode scalars.
    let doc = make_doc_with_content("日本語");
    let mut state = ProjectorState::new(Uuid::new_v4(), "日本語");
    let hash = state.last_projected_hash;

    // Append an ASCII character — only one insertion needed.
    ingest_file_change(&mut state, &doc, hash, "日本語!").unwrap();

    let locked = doc.lock().unwrap();
    assert_eq!(locked.get_text("content").to_string(), "日本語!");
}
