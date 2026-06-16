//! Localhost permission gate (US2 cross-process channel).
//!
//! Binds the loopback server, writes the MCP gate script per-run, mints a
//! per-run auth token, and returns the [`StartedGate`] the shell passes to the
//! CLI runner. The transport (`serve`) and policy (`handle_permission`) live in
//! `wagner_edge_host::permission_server` and are tested headlessly there; this
//! module is the thin Tauri shell glue that wires them together.

use wagner_edge_host::transmissions::TransmissionRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;

/// The MCP gate script (shipped in the binary) the app writes to disk per run.
const GATE_SERVER_SRC: &str = include_str!("../../host/resources/gate-server.mjs");

/// A started permission gate: the loopback server task plus the
/// [`wagner_edge_host::cli::GateConfig`] to hand the Claude runner.
pub struct StartedGate {
    pub config: wagner_edge_host::cli::GateConfig,
    pub server_task: tauri::async_runtime::JoinHandle<()>,
}

/// Bind the loopback permission server, write the MCP gate script to disk, mint a
/// per-run auth token, and build the gate config (US2). Permission requests open
/// transmissions on `reg` and emit `wagner://transmission` to the floor; the
/// server rejects any request lacking the token (M2). Kept here, with the
/// transport + policy, rather than in the Tauri command module.
pub async fn start_gate_server(
    app: &tauri::AppHandle,
    app_data: &std::path::Path,
    reg: Arc<TransmissionRegistry>,
    blocked_timeout_secs: u64,
    blocked_halt: Arc<std::sync::atomic::AtomicBool>,
) -> Result<StartedGate, String> {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("gate server bind failed: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    // Per-run secret: only a gate started by us and carrying this token can approve.
    let token = ulid::Ulid::new().to_string();

    // Write the shipped gate script next to the app's data so Claude can spawn it,
    // owner-only (0600) so another local user can't read or replace it.
    std::fs::create_dir_all(app_data).map_err(|e| e.to_string())?;
    let gate_script = app_data.join("gate-server.mjs");
    std::fs::write(&gate_script, GATE_SERVER_SRC).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&gate_script, std::fs::Permissions::from_mode(0o600));
    }

    let gate_url = format!("http://127.0.0.1:{port}/permission");
    let config = wagner_edge_host::cli::GateConfig {
        mcp_config_json: wagner_edge_host::cli::gate_mcp_config(
            &gate_script.to_string_lossy(),
            &gate_url,
            &token,
        ),
        prompt_tool: wagner_edge_host::cli::GATE_PROMPT_TOOL.to_string(),
    };

    let app_for_emit = app.clone();
    let handler = move |args: Value| {
        let reg = reg.clone();
        let app = app_for_emit.clone();
        let blocked_halt = blocked_halt.clone();
        async move {
            let emit = move |tj: Value| {
                let _ = app.emit("wagner://transmission", tj);
            };
            let on_timeout = move || blocked_halt.store(true, Ordering::SeqCst);
            let raised_at = chrono::Utc::now().to_rfc3339();
            wagner_edge_host::permission_server::handle_permission(
                &reg,
                &emit,
                &raised_at,
                args,
                blocked_timeout_secs,
                &on_timeout,
            )
            .await
        }
    };
    let server_task =
        tauri::async_runtime::spawn(wagner_edge_host::permission_server::serve(listener, handler, token));

    Ok(StartedGate { config, server_task })
}
