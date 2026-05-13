pub mod capture;
pub mod clipboard;
pub mod config;
pub mod error;
pub mod fs;
pub mod hotkey;
pub mod ipc;
pub mod logging;
pub mod pin;
pub mod state;
pub mod tray;

use crate::config::{defaults, store};
use crate::hotkey::{HotkeyKind, HotkeyService};
use crate::ipc::commands::{
    cancel_edit, finish_action, get_config, pin_close, reframe_request, selection_cancelled,
    selection_confirmed, update_config,
};
use crate::state::AppState;
use tauri::{Emitter, Listener, Manager, PhysicalPosition, WindowEvent};

/// Far off-screen position where the overlay window parks when not capturing.
/// Win32 window coordinates are i32; (-32000, -32000) is well outside any real
/// virtual desktop and is the convention Windows itself uses for "hidden but
/// alive" windows (see GetWindowPlacement). Keeping the overlay always visible
/// at this position avoids the 100ms WebView2 first-paint flicker that occurs
/// on every hide → show transition.
const OVERLAY_PARK_POS: PhysicalPosition<i32> = PhysicalPosition { x: -32000, y: -32000 };

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

            // Register both configured hotkeys (capture + paste-pin) and keep
            // handle in AppState so update_config can re-register at runtime.
            let state: tauri::State<AppState> = app.state();
            let (capture_combo, paste_combo) = {
                let cfg = state.config.lock().unwrap();
                (cfg.hotkey.clone(), cfg.paste_pin_hotkey.clone())
            };
            match crate::hotkey::PlatformHotkey::new() {
                Ok(mut hk) => {
                    if let Err(e) = hk.register(HotkeyKind::Capture, &capture_combo) {
                        tracing::warn!("capture hotkey '{}' conflict: {}", capture_combo, e);
                        let _ = app.emit(
                            "hotkey-conflict",
                            serde_json::json!({
                                "kind": "capture",
                                "attempted": capture_combo,
                                "reason": e.to_string(),
                            }),
                        );
                    }
                    if let Err(e) = hk.register(HotkeyKind::PastePin, &paste_combo) {
                        tracing::warn!("paste-pin hotkey '{}' conflict: {}", paste_combo, e);
                        let _ = app.emit(
                            "hotkey-conflict",
                            serde_json::json!({
                                "kind": "paste_pin",
                                "attempted": paste_combo,
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

            // Park the overlay window off-screen and show it once. From now on
            // the window stays visible (at -32000,-32000 when idle); capture
            // simply repositions it instead of toggling hide/show — eliminates
            // WebView2's first-paint grey flicker.
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_position(OVERLAY_PARK_POS);
                let _ = overlay.show();
            }

            // Bridge the tray/hotkey "trigger capture" event into the capture pipeline.
            let app_handle = app.handle().clone();
            app.listen("tray://trigger-capture", move |_| {
                if let Err(e) = crate::capture::trigger::trigger_capture(&app_handle) {
                    let _ = app_handle.emit("capture-error", e);
                }
            });

            // Intercept window close: hide and reset phase instead of destroying,
            // so windows can be reused next time they're shown.
            for label in ["overlay", "settings"] {
                if let Some(win) = app.get_webview_window(label) {
                    let app_handle = app.handle().clone();
                    let label = label.to_string();
                    win.on_window_event(move |event| {
                        if let WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            if let Some(w) = app_handle.get_webview_window(&label) {
                                let _ = w.hide();
                            }
                            // Settings is unrelated to the capture phase machine;
                            // only editor/overlay close should reset capture state.
                            if label != "settings" {
                                let state: tauri::State<AppState> = app_handle.state();
                                let mut phase = state.phase.lock().unwrap();
                                let _ = phase.transition(crate::state::PhaseEvent::Cancelled);
                                *state.capture.lock().unwrap() = None;
                                *state.cropped.lock().unwrap() = None;
                            }
                            tracing::info!("window '{}' close intercepted", label);
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
            reframe_request,
            finish_action,
            cancel_edit,
            pin_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
