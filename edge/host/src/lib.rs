//! Wagner edge host (platform wedge).
//!
//! Ported incrementally from `apps/wagner/src-tauri` per the wedge plan
//! (`platform/specs/001-shared-runs-and-learnings/`). Modules land per task:
//!   - `project_key` derivation (T009b/T014b)
//!   - durable sync queue + hub client (T022/T023)
//!   - OIDC Authorization-Code + PKCE flow (T013)
//!   - orchestrator port that emits the event-sourced run-event log (T024; audit F1)
//!
//! Article VII: nothing outside `platform/` depends on this crate.

// Modules are added per task; the crate compiles empty until then.
