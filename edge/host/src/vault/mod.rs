//! The Vault — the linked-markdown knowledge layer over the memory store
//! (Plan 004). Link/backlink extraction is deterministic code (this module); the
//! LLM is used only for semantic extraction (summary/tier/provenance) elsewhere.
//!
//! Phase 5 (006) extends this module with distributed CRDT sync: one Loro
//! document per note, a bidirectional file↔CRDT projector, and transport-agnostic
//! SyncAdapter / SnapshotStore traits.

pub mod crdt;
pub mod linker;
pub mod projector;
pub mod snapshot_store;
pub mod sync_adapter;

pub use linker::{parse_wikilinks, WikiLink};
