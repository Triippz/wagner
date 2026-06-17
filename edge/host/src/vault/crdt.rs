//! VaultCrdt — per-note Loro CRDT documents.
//!
//! Holds one `LoroDoc` per vault note UUID, identified by [`uuid::Uuid`].
//! The map itself is NOT `Send`; callers must ensure single-task or lock ownership.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use loro::{ExportMode, LoroDoc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum VaultCrdtError {
    #[error("note not found: {0}")]
    NoteNotFound(Uuid),
    #[error("loro export failed: {0}")]
    ExportFailed(String),
    #[error("loro import failed: {0}")]
    ImportFailed(String),
}

/// Manages in-memory Loro CRDT documents for vault notes.
#[derive(Default)]
pub struct VaultCrdt {
    docs: HashMap<Uuid, Arc<Mutex<LoroDoc>>>,
}

impl VaultCrdt {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure a note document exists. Idempotent — calling twice for the same UUID is a no-op.
    pub fn init_note(&mut self, uuid: Uuid) -> Result<(), VaultCrdtError> {
        self.docs.entry(uuid).or_insert_with(|| Arc::new(Mutex::new(LoroDoc::new())));
        Ok(())
    }

    /// Export the full snapshot bytes for a note.
    ///
    /// # Errors
    /// Returns [`VaultCrdtError::NoteNotFound`] if [`init_note`](Self::init_note) was never called.
    pub fn export_note(&self, uuid: Uuid) -> Result<Vec<u8>, VaultCrdtError> {
        let doc_arc = self.docs.get(&uuid).ok_or(VaultCrdtError::NoteNotFound(uuid))?;
        // ponytail: std::sync::Mutex OK — lock dropped before any .await
        let doc = doc_arc.lock().expect("VaultCrdt doc mutex poisoned");
        doc.export(ExportMode::Snapshot)
            .map_err(|e| VaultCrdtError::ExportFailed(e.to_string()))
    }

    /// Import (merge) a snapshot into a note's document.
    ///
    /// Creates the note document first if it does not exist.
    pub fn import_note(&mut self, uuid: Uuid, snapshot: Vec<u8>) -> Result<(), VaultCrdtError> {
        self.init_note(uuid)?;
        let doc_arc = Arc::clone(self.docs.get(&uuid).expect("just inserted"));
        // ponytail: std::sync::Mutex OK — lock dropped before any .await
        let doc = doc_arc.lock().expect("VaultCrdt doc mutex poisoned");
        doc.import(&snapshot)
            .map_err(|e| VaultCrdtError::ImportFailed(e.to_string()))?;
        Ok(())
    }

    /// Return a cloned `Arc` to the raw `LoroDoc` mutex for a note, if it exists.
    pub fn doc_arc(&self, uuid: Uuid) -> Option<Arc<Mutex<LoroDoc>>> {
        self.docs.get(&uuid).cloned()
    }
}
