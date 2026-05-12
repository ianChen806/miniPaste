use crate::capture::Rect;
use crate::config::Config;
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
pub fn get_config(_state: State<AppState>) -> Result<Config, AppError> {
    // Stub: returns error until Task 16 wires in the config cache.
    Err(AppError::Other("not yet wired".into()))
}

#[tauri::command]
pub fn update_config(_new: Config, _state: State<AppState>) -> Result<(), AppError> {
    Err(AppError::Other("not yet wired".into()))
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
