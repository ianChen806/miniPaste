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
use tauri::{Emitter, Manager};

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

            // Register the configured hotkey.
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
                    // Keep the hotkey registration alive for the process lifetime.
                    // (Task 32 will move this into AppState to support runtime re-register.)
                    Box::leak(Box::new(hk));
                }
                Err(e) => {
                    tracing::error!("hotkey init failed: {}", e);
                }
            }

            crate::hotkey::listener::spawn(app.handle().clone());
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
