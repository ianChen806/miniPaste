pub mod capture;
pub mod clipboard;
pub mod config;
pub mod error;
pub mod fs;
pub mod hotkey;
pub mod ipc;
pub mod logging;
pub mod state;
pub mod tray;

use crate::config::{defaults, store};
use crate::hotkey::HotkeyService;
use crate::ipc::commands::{
    cancel_edit, finish_action, get_config, selection_cancelled, selection_confirmed,
    update_config,
};
use crate::state::AppState;
use tauri::{Emitter, Listener, Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_data = dirs::config_dir()
        .expect("config dir not available")
        .join("minipaste");
    let config_path = store::config_path(app_data);
    let config = store::load_or_init(&config_path).unwrap_or_else(|_| defaults::default_config());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(config, config_path))
        .setup(|app| {
            crate::tray::build_tray(app.handle())?;

            // Register the configured hotkey and keep handle in AppState
            // so update_config can re-register at runtime.
            let state: tauri::State<AppState> = app.state();
            let combo = state.config.lock().unwrap().hotkey.clone();
            match crate::hotkey::PlatformHotkey::new() {
                Ok(mut hk) => {
                    if let Err(e) = hk.register(&combo) {
                        let _ = app.emit(
                            "hotkey-conflict",
                            serde_json::json!({
                                "attempted": combo,
                                "reason": e.to_string(),
                            }),
                        );
                    }
                    *state.hotkey.lock().unwrap() = Some(hk);
                }
                Err(e) => {
                    tracing::error!("hotkey init failed: {}", e);
                }
            }

            crate::hotkey::listener::spawn(app.handle().clone());

            // Bridge the tray/hotkey "trigger capture" event into the capture pipeline.
            let app_handle = app.handle().clone();
            app.listen("tray://trigger-capture", move |_| {
                if let Err(e) = crate::capture::trigger::trigger_capture(&app_handle) {
                    let _ = app_handle.emit("capture-error", e);
                }
            });

            // Intercept editor/overlay/settings window close: hide and reset phase
            // instead of destroying, so the windows can be reused next time.
            for label in ["editor", "overlay"] {
                if let Some(win) = app.get_webview_window(label) {
                    let app_handle = app.handle().clone();
                    let label = label.to_string();
                    win.on_window_event(move |event| {
                        if let WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            if let Some(w) = app_handle.get_webview_window(&label) {
                                let _ = w.hide();
                            }
                            let state: tauri::State<AppState> = app_handle.state();
                            let mut phase = state.phase.lock().unwrap();
                            let _ = phase.transition(crate::state::PhaseEvent::Cancelled);
                            *state.capture.lock().unwrap() = None;
                            *state.cropped.lock().unwrap() = None;
                            tracing::info!("window '{}' close intercepted; phase reset", label);
                        }
                    });
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            selection_confirmed,
            selection_cancelled,
            finish_action,
            cancel_edit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
