//! Wagner Edge shell — Tauri entry point.
//!
//! This crate is the thin Tauri 2 shell that wires `wagner-edge-host` (a
//! headless library) to a real desktop window: IPC commands, tray lifecycle,
//! loopback permission gate, and macOS window-hide behaviour.

pub mod commands;
pub mod gate;
pub mod pool;
pub mod suite;
pub mod voice_lifecycle;

use std::sync::Arc;
use tauri::Manager;
use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent, WindowAction};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(commands::RunManager::default())
        .manage(Arc::new(
            wagner_edge_host::transmissions::TransmissionRegistry::default(),
        ))
        .manage(wagner_edge_host::voice::VoiceManager::new())
        .manage(voice_lifecycle::SidecarState::new())
        .setup(|app| {
            // Open (or create) the persistent memory store under the app-data dir.
            // Done synchronously (block_on) so the store is managed BEFORE any
            // command can run — managing it from a spawned task would race the
            // first `save_memory`/`recall_memory`/`start_run` invocation, and the
            // commands take `State<'_, MemoryStore>` (not a Mutex wrapper).
            let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&app_data)?;
            let store = tauri::async_runtime::block_on(
                wagner_edge_host::memory::MemoryStore::open(&app_data.join("memory-db"), "local"),
            )?;
            app.manage(store);

            // macOS: run as an accessory (no Dock icon) so the tray is the only
            // visible anchor.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Track window + host lifecycle for close-to-hide behaviour (FR-101/104).
            app.manage(std::sync::Mutex::new(HostLifecycle::new()));

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close → hide (T019): prevent destroy, just hide the window so the
            // tray + host keep running in the background.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let lc = window.state::<std::sync::Mutex<HostLifecycle>>();
                let action = lc.lock().unwrap().on_event(LifecycleEvent::CloseRequested);
                if action == WindowAction::Hide {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::preflight,
            commands::ping_endpoint,
            commands::agent_catalog,
            commands::skill_catalog,
            commands::validate_project_dir,
            commands::start_run,
            commands::list_workflow_templates,
            commands::save_workflow_template,
            commands::save_memory,
            commands::recall_memory,
            commands::validate_workflow,
            commands::start_workflow,
            commands::steer,
            commands::answer_transmission,
            commands::abort,
            commands::list_runs,
            commands::get_run,
            commands::resume_run,
            commands::add_goal,
            commands::vault_summary,
            commands::approve_staging,
            commands::list_staging,
            commands::vault_graph,
            commands::voice_status,
            commands::voice_set_enabled,
            commands::voice_models_status,
            commands::voice_download_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running wagner-edge-shell");
}
