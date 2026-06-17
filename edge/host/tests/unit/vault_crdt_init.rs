//! US-001 — VaultCrdt init, export, and convergence tests.

use loro::{ExportMode, LoroDoc};
use uuid::Uuid;
use wagner_edge_host::vault::crdt::{VaultCrdt, VaultCrdtError};

#[test]
fn test_init_note_creates_doc() {
    let mut crdt = VaultCrdt::new();
    let id = Uuid::new_v4();
    crdt.init_note(id).expect("init must succeed");
    let bytes = crdt.export_note(id).expect("export must succeed after init");
    assert!(!bytes.is_empty(), "snapshot must be non-empty");
}

#[test]
fn test_init_note_is_idempotent() {
    let mut crdt = VaultCrdt::new();
    let id = Uuid::new_v4();
    crdt.init_note(id).expect("first init");
    crdt.init_note(id).expect("second init must not error");
    crdt.export_note(id).expect("export after double-init must succeed");
}

#[test]
fn test_export_unknown_uuid_errors() {
    let crdt = VaultCrdt::new();
    let id = Uuid::new_v4();
    let result = crdt.export_note(id);
    assert!(
        matches!(result, Err(VaultCrdtError::NoteNotFound(_))),
        "export of uninitialised uuid must return NoteNotFound"
    );
}

#[test]
fn test_convergence_two_peers() {
    let mut peer_a = VaultCrdt::new();
    let mut peer_b = VaultCrdt::new();
    let id = Uuid::new_v4();

    peer_a.init_note(id).unwrap();
    peer_b.init_note(id).unwrap();

    // Peer A writes "hello world" to the content text container.
    {
        let doc_arc = peer_a.doc_arc(id).unwrap();
        let doc = doc_arc.lock().unwrap();
        doc.get_text("content").insert(0, "hello world").unwrap();
        doc.commit();
    }

    // Peer A exports snapshot and peer B imports it.
    let snapshot = peer_a.export_note(id).unwrap();
    peer_b.import_note(id, snapshot).unwrap();

    // Both peers should now have the same text content.
    let b_content = {
        let doc_arc = peer_b.doc_arc(id).unwrap();
        let doc = doc_arc.lock().unwrap();
        doc.get_text("content").to_string()
    };

    assert_eq!(b_content, "hello world", "peer B must converge to peer A's content");
}

/// Test Fugue CRDT convergence: two peers independently insert text, exchange ops,
/// and end up with identical (deterministic) content.
#[test]
fn test_concurrent_convergence() {
    // Two independent LoroDoc instances — no VaultCrdt wrapper needed, testing the CRDT property.
    let doc_a = LoroDoc::new();
    let doc_b = LoroDoc::new();
    doc_a.set_peer_id(1).unwrap();
    doc_b.set_peer_id(2).unwrap();

    let text_a = doc_a.get_text("content");
    let text_b = doc_b.get_text("content");

    // A inserts "foo" at 0; B inserts "bar" at 0.
    text_a.insert(0, "foo").unwrap();
    doc_a.commit();

    text_b.insert(0, "bar").unwrap();
    doc_b.commit();

    // Exchange all updates.
    let updates_a = doc_a.export(ExportMode::all_updates()).unwrap();
    let updates_b = doc_b.export(ExportMode::all_updates()).unwrap();

    doc_a.import(&updates_b).unwrap();
    doc_b.import(&updates_a).unwrap();

    // After convergence both docs must have identical text.
    let result_a = doc_a.get_text("content").to_string();
    let result_b = doc_b.get_text("content").to_string();

    assert_eq!(result_a, result_b, "Fugue convergence: both peers must have identical text");
    assert!(!result_a.is_empty(), "merged text must contain both insertions");
}
