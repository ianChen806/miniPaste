use crate::config::{store, Config};
use crate::error::AppError;
use crate::state::{AppState, PhaseEvent};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};

/// Off-screen position where the overlay parks when idle. Must match
/// `OVERLAY_PARK_POS` in lib.rs.
const OVERLAY_PARK_POS: PhysicalPosition<i32> = PhysicalPosition { x: -32000, y: -32000 };

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FinishAction {
    CopyImage,
    Save { path: PathBuf },
    SaveAndCopyPath,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinishOutcome {
    pub saved_path: Option<PathBuf>,
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, AppError> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn update_config(
    new: Config,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    use crate::hotkey::{HotkeyKind, HotkeyService};

    let old = state.config.lock().unwrap().clone();
    let capture_changed = new.hotkey != old.hotkey;
    let paste_changed = new.paste_pin_hotkey != old.paste_pin_hotkey;

    if capture_changed || paste_changed {
        let mut hk_slot = state.hotkey.lock().unwrap();
        if let Some(hk) = hk_slot.as_mut() {
            if capture_changed {
                if let Err(e) = hk.register(HotkeyKind::Capture, &new.hotkey) {
                    let _ = app.emit(
                        "hotkey-conflict",
                        serde_json::json!({
                            "kind": "capture",
                            "attempted": new.hotkey,
                            "reason": e.to_string(),
                        }),
                    );
                    let _ = hk.register(HotkeyKind::Capture, &old.hotkey);
                    return Err(e.into());
                }
            }
            if paste_changed {
                if let Err(e) = hk.register(HotkeyKind::PastePin, &new.paste_pin_hotkey) {
                    let _ = app.emit(
                        "hotkey-conflict",
                        serde_json::json!({
                            "kind": "paste_pin",
                            "attempted": new.paste_pin_hotkey,
                            "reason": e.to_string(),
                        }),
                    );
                    let _ = hk.register(HotkeyKind::PastePin, &old.paste_pin_hotkey);
                    return Err(e.into());
                }
            }
        }
    }

    store::save(&state.config_path, &new)?;
    *state.config.lock().unwrap() = new;
    Ok(())
}

#[tauri::command]
pub fn selection_confirmed(state: State<AppState>) -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    phase
        .transition(PhaseEvent::SelectionConfirmed)
        .map_err(|e| AppError::State(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn selection_cancelled(
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::Cancelled);
    }
    *state.capture.lock().unwrap() = None;
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("capture-clear", ());
        let _ = overlay.set_always_on_top(false);
        let _ = overlay.set_position(OVERLAY_PARK_POS);
    }
    Ok(())
}

#[tauri::command]
pub fn finish_action(
    action: FinishAction,
    image_bytes: Vec<u8>,
    state: State<AppState>,
    app: AppHandle,
) -> Result<FinishOutcome, AppError> {
    use crate::clipboard::{Clipboard, PlatformClipboard};
    let clipboard = PlatformClipboard::new();
    match action {
        FinishAction::CopyImage => {
            clipboard.write_image(&image_bytes)?;
            finalize(&app, &state, FinishOutcome { saved_path: None })
        }
        FinishAction::Save { path } => {
            crate::fs::save::write_image_file(&path, &image_bytes)?;
            *state.last_save_dir.lock().unwrap() =
                path.parent().map(|p| p.to_path_buf());
            finalize(
                &app,
                &state,
                FinishOutcome {
                    saved_path: Some(path),
                },
            )
        }
        FinishAction::SaveAndCopyPath => {
            let cfg = state.config.lock().unwrap().clone();
            crate::fs::save::validate_writable_dir(&cfg.default_save_path)?;
            let filename =
                crate::fs::filename::now_filename(cfg.image_format.extension());
            let path = cfg.default_save_path.join(filename);
            crate::fs::save::write_image_file(&path, &image_bytes)?;
            if let Err(e) = clipboard.write_file_paths(&[path.clone()]) {
                tracing::error!("save_and_copy: clipboard write failed: {}", e);
                return Err(e.into());
            }
            *state.last_save_dir.lock().unwrap() =
                path.parent().map(|p| p.to_path_buf());
            finalize(
                &app,
                &state,
                FinishOutcome {
                    saved_path: Some(path),
                },
            )
        }
    }
}

fn finalize(
    app: &AppHandle,
    state: &State<AppState>,
    outcome: FinishOutcome,
) -> Result<FinishOutcome, AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::ActionFinished);
    }
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("capture-clear", ());
        let _ = overlay.set_always_on_top(false);
        let _ = overlay.set_position(OVERLAY_PARK_POS);
    }
    let _ = app.emit(
        "action-complete",
        serde_json::json!({ "saved_path": outcome.saved_path }),
    );
    Ok(outcome)
}

#[tauri::command]
pub fn pin_close(
    label: String,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.close();
    }
    state.pins.release(&label);
    Ok(())
}

#[tauri::command]
pub fn cancel_edit(
    state: State<AppState>,
    _app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::Cancelled);
    }
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn reframe_request(state: State<AppState>) -> Result<(), AppError> {
    let mut phase = state.phase.lock().unwrap();
    phase
        .transition(PhaseEvent::ReframeRequest)
        .map_err(|e| AppError::State(e.to_string()))?;
    Ok(())
}
