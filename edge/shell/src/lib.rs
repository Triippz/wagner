//! Wagner Edge shell — Tauri entry point.
//!
//! This crate is the thin Tauri 2 shell that wires `wagner-edge-host` (a
//! headless library) to a real desktop window: IPC commands, tray lifecycle,
//! loopback permission gate, and macOS window-hide behaviour.

pub mod bus_gateway;
pub mod commands;
pub mod gate;
pub mod pool;
pub mod suite;
pub mod voice_lifecycle;

use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent, WindowAction};

/// Compute the collection behavior that lets the green title-bar button enter a
/// true macOS fullscreen Space (F38): clear the two flags that pin the button to
/// "zoom" (the partial maximize the operator was seeing) and set FullScreenPrimary.
/// Pure so it is unit-tested without AppKit.
#[cfg(target_os = "macos")]
fn with_fullscreen_primary(
    current: objc2_app_kit::NSWindowCollectionBehavior,
) -> objc2_app_kit::NSWindowCollectionBehavior {
    use objc2_app_kit::NSWindowCollectionBehavior as B;
    let cleared = current.0 & !B::FullScreenAuxiliary.0 & !B::FullScreenNone.0;
    B(cleared | B::FullScreenPrimary.0)
}

/// Show the main window and put the app in the Dock. The Dock icon is wanted only
/// while the window is open, so showing flips the activation policy to `Regular`.
/// Used on launch and when the tray brings a hidden window back.
fn show_main_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Hide the main window to the tray and drop the Dock icon (`Accessory`). While
/// hidden the tray icon is the only anchor — no Dock icon, no Cmd-Tab entry.
fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(
            wagner_edge_host::transmissions::TransmissionRegistry::default(),
        ))
        .manage(Arc::new(wagner_edge_host::voice::VoiceManager::new()))
        .manage(voice_lifecycle::SidecarState::new())
        .manage(commands::PttState::new())
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

            // Event bus (011 P1/P2): the UiGateway re-emits typed bus events to
            // the legacy wagner://* Tauri channels, so the React surface is
            // unchanged while the emit side migrates onto the bus. Spawned before
            // any run can publish.
            let bus = Arc::new(wagner_edge_host::bus::Bus::new(1024));
            bus_gateway::spawn(bus.clone(), app.handle().clone());
            // 014 US1: the AgentRegistry is the single authority for live runs
            // (replaces the shell's RunManager). Same bus as the UiGateway so the
            // loop's published facts reach the UI.
            let registry = Arc::new(wagner_edge_host::bus::AgentRegistry::new(bus.clone()));
            // 015 T014b: register the voice intake participant — a published
            // `voice.utterance_transcribed` → route_transcript → dispatch a RunCommand
            // → RunCommandRouter → run. Shares the managed VoiceManager so the toggle
            // gate (FR-015) + typed-error surfacing (FR-014) apply.
            {
                use wagner_edge_host::bus::{AllowAll, NodeId, ParticipantId, ParticipantKind};
                let voice = app
                    .state::<Arc<wagner_edge_host::voice::VoiceManager>>()
                    .inner()
                    .clone();
                let ctx = registry.context(ParticipantId {
                    node: NodeId("local".into()),
                    kind: ParticipantKind::Agent,
                    name: "voice-intake".into(),
                    instance: ulid::Ulid::new(),
                });
                let intake =
                    wagner_edge_host::participants::VoiceIntake::new(ctx, Arc::new(AllowAll), voice);
                // `registry.spawn` uses `tokio::spawn` internally; enter the runtime.
                tauri::async_runtime::block_on(async { registry.spawn(Box::new(intake)) });
            }
            app.manage(registry);
            app.manage(bus_gateway::UiGateway::new(bus, app.handle().clone()));

            // System tray — the visible anchor when the window is hidden. Built
            // here (not in tauri.conf) so its handlers can flip the Dock icon with
            // the window: shown → Regular (Dock icon), hidden → Accessory (none).
            // Left-click (and the Show item) bring the window back; Quit exits.
            let show_item = MenuItem::with_id(app.handle(), "show", "Show Wagner", true, None::<&str>)?;
            let quit_item = PredefinedMenuItem::quit(app.handle(), Some("Quit Wagner"))?;
            let tray_menu = Menu::with_items(app.handle(), &[&show_item, &quit_item])?;
            let mut tray = TrayIconBuilder::with_id("wagner-tray")
                .tooltip("Wagner Edge")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "show" {
                        show_main_window(app);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Left-click brings the window back (and the Dock icon with it).
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });
            // ponytail: reuse the colored app icon; a monochrome template glyph
            // would sit better in the macOS menu bar — swap in later.
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            // Keep the tray alive — it is ref-counted and vanishes if dropped.
            app.manage(tray.build(app.handle())?);

            // F38: force FullScreenPrimary so the green title-bar button enters a
            // true fullscreen Space (a window that can go accessory does not get it
            // by default — the green button only "zooms"). Harmless under Regular.
            #[cfg(target_os = "macos")]
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(ptr) = win.ns_window() {
                    use objc2_app_kit::NSWindow;
                    // SAFETY: Tauri returns a valid, retained NSWindow pointer for
                    // the live "main" window; `.setup` runs on the main thread (the
                    // only place AppKit window state may be mutated) and the window
                    // outlives this borrow.
                    let ns_window: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
                    ns_window
                        .setCollectionBehavior(with_fullscreen_primary(ns_window.collectionBehavior()));
                }
            }

            // Show the window on launch → Regular activation, so the Dock icon is
            // present and the window is focused (it is hidden to the tray on close).
            show_main_window(app.handle());

            // Track window + host lifecycle for close-to-hide behaviour (FR-101/104).
            app.manage(std::sync::Mutex::new(HostLifecycle::new()));

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close → hide (T019): prevent destroy, hide the window AND drop the
            // Dock icon (Accessory) so the tray is the only anchor while hidden;
            // the host keeps running in the background.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let lc = window.state::<std::sync::Mutex<HostLifecycle>>();
                let action = lc.lock().unwrap().on_event(LifecycleEvent::CloseRequested);
                if action == WindowAction::Hide {
                    api.prevent_close();
                    hide_main_window(window.app_handle());
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
            commands::voice_ptt_start,
            commands::voice_ptt_stop,
            commands::voice_models_status,
            commands::voice_download_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running wagner-edge-shell");
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::with_fullscreen_primary;
    use objc2_app_kit::NSWindowCollectionBehavior as B;

    #[test]
    fn fullscreen_primary_replaces_zoom_behaviors() {
        // "No fullscreen" → green button is zoom; force a real fullscreen Space.
        assert_eq!(with_fullscreen_primary(B::FullScreenNone).0, B::FullScreenPrimary.0);
        // "Auxiliary" (can only join another app's space) → Primary.
        assert_eq!(with_fullscreen_primary(B::FullScreenAuxiliary).0, B::FullScreenPrimary.0);
        // Already Primary → unchanged (idempotent re-application).
        assert_eq!(with_fullscreen_primary(B::FullScreenPrimary).0, B::FullScreenPrimary.0);
        // Unrelated behavior bits are preserved.
        let extra = B(B::FullScreenNone.0 | 0b1);
        assert_eq!(with_fullscreen_primary(extra).0, B::FullScreenPrimary.0 | 0b1);
    }
}
