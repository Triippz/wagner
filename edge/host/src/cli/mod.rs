//! CLI integration: spawning and driving the `claude`/`codex` subprocesses.

pub mod driver;
pub mod endpoint;
pub mod preflight;
pub mod runner;

pub use driver::{Driver, DriverError};
pub use endpoint::{ping as ping_endpoint, EndpointRunner, EndpointStatus};
pub use preflight::{detect_system, CliStatus, EngineStatus};
pub use runner::{gate_mcp_config, CliEngineRunner, GateConfig, GATE_PROMPT_TOOL};
