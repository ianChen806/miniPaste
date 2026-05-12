use crate::capture::Rect;
use crate::config::{store, Config};
use crate::error::AppError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};

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
    _rect: Rect,
    _state: State<AppState>,
    _app: AppHandle,
) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn selection_cancelled(_state: State<AppState>, _app: AppHandle) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
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
pub fn cancel_edit(_state: State<AppState>, _app: AppHandle) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
}
