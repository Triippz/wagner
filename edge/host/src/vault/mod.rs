//! The Vault — the linked-markdown knowledge layer over the memory store
//! (Plan 004). Link/backlink extraction is deterministic code (this module); the
//! LLM is used only for semantic extraction (summary/tier/provenance) elsewhere.

pub mod linker;

pub use linker::{parse_wikilinks, WikiLink};
