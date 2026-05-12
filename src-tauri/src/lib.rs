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

use crate::ipc::commands::{
    cancel_edit, finish_action, get_config, selection_cancelled, selection_confirmed,
    update_config,
};
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
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
