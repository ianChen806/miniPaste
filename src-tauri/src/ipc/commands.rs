use crate::capture::{Capture, PlatformCapture, Rect};
use crate::config::{store, Config};
use crate::error::AppError;
use crate::state::{AppState, PhaseEvent};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

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
    _app: AppHandle,
) -> Result<(), AppError> {
    // Persist first; if write fails, do not update in-memory state.
    // Hotkey re-registration is wired in Task 32.
    store::save(&state.config_path, &new)?;
    *state.config.lock().unwrap() = new;
    Ok(())
}

#[tauri::command]
pub fn selection_confirmed(
    rect: Rect,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        phase
            .transition(PhaseEvent::SelectionConfirmed)
            .map_err(|e| AppError::State(e.to_string()))?;
    }

    let frame_opt = state.capture.lock().unwrap().clone();
    let frame =
        frame_opt.ok_or_else(|| AppError::Capture("no frame in state".into()))?;

    let cap = PlatformCapture::new();
    let cropped = cap.crop(&frame, rect)?;
    *state.cropped.lock().unwrap() = Some(cropped.clone());

    // Hide overlay, open editor with cropped image.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    if let Some(editor) = app.get_webview_window("editor") {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&cropped);
        let _ = editor.show();
        let _ = editor.set_focus();
        let _ = editor.emit(
            "editor-ready",
            serde_json::json!({
                "image_b64": b64,
                "width": rect.w,
                "height": rect.h,
            }),
        );
    }
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
        let _ = overlay.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn finish_action(
    _action: FinishAction,
    _image_bytes: Vec<u8>,
    _state: State<AppState>,
    _app: AppHandle,
) -> Result<FinishOutcome, AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn cancel_edit(
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    {
        let mut phase = state.phase.lock().unwrap();
        let _ = phase.transition(PhaseEvent::Cancelled);
    }
    *state.cropped.lock().unwrap() = None;
    *state.capture.lock().unwrap() = None;
    if let Some(editor) = app.get_webview_window("editor") {
        let _ = editor.hide();
    }
    Ok(())
}
