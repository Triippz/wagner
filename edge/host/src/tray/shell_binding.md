# T019 — Tauri shell binding (live-integration reference)

The tray + window lifecycle **logic** lives in `tray/mod.rs` and is fully unit-
tested headlessly (T015 `tray_lifecycle`, T016 `run_survives_close`, T017
`tray_status`, T018 `reopen_fidelity`). This file specifies the thin Tauri shell
glue that wires the live desktop app to that tested logic.

It is **not compiled into the headless `wagner-edge-host` lib** (Article VI: the
host stays Tauri-independent so the same crate later runs as a headless daemon —
plan R-5). It lands when the desktop app bundle is ported from `apps/wagner/
src-tauri` (tauri.conf.json, build.rs, capabilities, icons, the built `edge/ui`
Vite dist). Every line below calls a function already covered by a passing test.

## Window lifecycle → `HostLifecycle`

```rust
use wagner_edge_host::tray::{HostLifecycle, LifecycleEvent, WindowAction};

tauri::Builder::default()
    // macOS: live in the menu bar, no Dock icon — the host outlives the window.
    .setup(|app| {
        #[cfg(target_os = "macos")]
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        app.manage(std::sync::Mutex::new(HostLifecycle::new()));
        Ok(())
    })
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let lc = window.state::<std::sync::Mutex<HostLifecycle>>();
            let action = lc.lock().unwrap().on_event(LifecycleEvent::CloseRequested);
            if action == WindowAction::Hide {
                api.prevent_close();      // hide, never exit (T015)
                let _ = window.hide();
            }
        }
    })
```

App-quit (`Cmd-Q` / menu) calls `on_event(LifecycleEvent::AppQuit)` → `Exit`,
the one explicit host teardown (FR-104). Tray-click → `LifecycleEvent::Reopen` →
`window.show()`.

## Tray status → `derive_status` / `present` / `notification_on_transition`

On every run-state change the shell recomputes and applies the tested mapping:

```rust
use wagner_edge_host::tray::{derive_status, present, notification_on_transition};

let next = derive_status(run.status, registry.open_count());
let pres = present(next);
tray_icon.set_title(Some(format!("{} {}", pres.glyph, pres.label)));  // non-color (T017)
tray_icon.set_icon(Some(badge_icon(pres.badge)));
if let Some(note) = notification_on_transition(prev_status, next) {
    // SC-008: fires within 5 s of entering needs-you, exactly once.
    tauri_plugin_notification::Notification::new(app_id)
        .title(note.title).body(note.body).show()?;
}
prev_status = next;
```

## Surface delivery

The desktop webview loads the unified `edge/ui` surface, which folds the host log
over the **IPC transport adapter** (`edge/ui/transport/ipc.ts`, T021 — tested with
a fake bridge). The shell provides the real `TauriBridge` (`@tauri-apps/api`
`listen`/`invoke`) at boot; `createTransport({hasTauri:true}, bridge)` selects it.

## Verification status

- Logic: **tested headlessly** (10 Rust tests + the surface/IPC vitest suite).
- Binding: **integration-only** — verified live by running the ported desktop
  app, the same way `apps/wagner` proves this Tauri pattern today. Not in CI
  (Tauri lifecycle needs tauri-driver/WebDriver, poor macOS support).
