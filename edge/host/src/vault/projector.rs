//! Projector — bidirectional sync between the filesystem and the Loro CRDT.
//!
//! File→CRDT: [`ingest_file_change`] applies a diff of new content onto a [`LoroDoc`].
//! CRDT→File: [`flush_to_file`] writes current CRDT text back if content changed.

use std::sync::Mutex;

use loro::LoroDoc;
use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProjectorError {
    #[error("stale hash: provided hash does not match current state")]
    StaleHash,
    #[error("note not found: {0}")]
    NoteNotFound(Uuid),
    #[error("loro error: {0}")]
    Loro(String),
}

/// Per-note projector state.
pub struct ProjectorState {
    pub uuid: Uuid,
    pub last_projected_hash: [u8; 32],
    pub edit_in_flight: bool,
}

impl ProjectorState {
    pub fn new(uuid: Uuid, initial_content: &str) -> Self {
        Self {
            uuid,
            last_projected_hash: sha256_bytes(initial_content.as_bytes()),
            edit_in_flight: false,
        }
    }
}

/// The result of a [`flush_to_file`] call.
pub enum FlushResult {
    /// Content changed; the new content string is returned.
    Written(String),
    /// No write needed (`edit_in_flight` was set OR content was unchanged).
    Skipped,
}

/// SHA-256 hash of raw bytes.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Apply character-level diff of `before → after` as insert/delete ops on `text`.
pub fn diff_to_ops(
    before: &str,
    after: &str,
    text: &loro::LoroText,
) -> Result<(), ProjectorError> {
    let diff = TextDiff::from_chars(before, after);
    let mut pos: usize = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                pos += change.value().chars().count();
            }
            ChangeTag::Insert => {
                let s = change.value();
                text.insert(pos, s)
                    .map_err(|e| ProjectorError::Loro(e.to_string()))?;
                pos += s.chars().count();
            }
            ChangeTag::Delete => {
                let len = change.value().chars().count();
                text.delete(pos, len)
                    .map_err(|e| ProjectorError::Loro(e.to_string()))?;
                // pos does not advance on delete
            }
        }
    }
    Ok(())
}

/// Ingest a file change into the CRDT document.
///
/// `caller_hash` must match `state.last_projected_hash`; mismatches return [`ProjectorError::StaleHash`].
///
/// # Ponytail note
/// The Mutex lock is acquired, the diff applied, and the lock released before any async work.
/// ponytail: std::sync::Mutex OK — lock dropped before any .await
pub fn ingest_file_change(
    state: &mut ProjectorState,
    doc: &Mutex<LoroDoc>,
    caller_hash: [u8; 32],
    new_content: &str,
) -> Result<(), ProjectorError> {
    if caller_hash != state.last_projected_hash {
        return Err(ProjectorError::StaleHash);
    }

    // ponytail: std::sync::Mutex OK — lock dropped before any .await
    let locked = doc.lock().expect("projector doc mutex poisoned");
    let text = locked.get_text("content");

    let current_text = text.to_string();
    diff_to_ops(&current_text, new_content, &text)?;
    locked.commit();
    drop(locked);

    state.last_projected_hash = sha256_bytes(new_content.as_bytes());
    state.edit_in_flight = true;
    Ok(())
}

/// Flush the current CRDT text content to the file, if changed.
///
/// Returns [`FlushResult::Skipped`] when `state.edit_in_flight` is true (a local edit is in
/// progress) or when the content has not changed since the last projection.
///
/// # Ponytail note
/// ponytail: std::sync::Mutex OK — lock dropped before any .await
pub fn flush_to_file(
    state: &mut ProjectorState,
    doc: &Mutex<LoroDoc>,
) -> Result<FlushResult, ProjectorError> {
    if state.edit_in_flight {
        return Ok(FlushResult::Skipped);
    }

    // ponytail: std::sync::Mutex OK — lock dropped before any .await
    let locked = doc.lock().expect("projector doc mutex poisoned");
    let content = locked.get_text("content").to_string();
    drop(locked);

    let content_hash = sha256_bytes(content.as_bytes());
    if content_hash == state.last_projected_hash {
        return Ok(FlushResult::Skipped);
    }

    state.last_projected_hash = content_hash;
    Ok(FlushResult::Written(content))
}
