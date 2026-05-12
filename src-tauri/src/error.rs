use crate::capture::CaptureError;
use crate::clipboard::ClipboardError;
use crate::config::store::ConfigError;
use crate::fs::save::SaveError;
use crate::hotkey::HotkeyError;
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Hotkey(String),
    #[error("{0}")]
    Capture(String),
    #[error("{0}")]
    Clipboard(String),
    #[error("{0}")]
    Save(String),
    #[error("{0}")]
    State(String),
    #[error("{0}")]
    Other(String),
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        Self::Config(e.to_string())
    }
}
impl From<CaptureError> for AppError {
    fn from(e: CaptureError) -> Self {
        Self::Capture(e.to_string())
    }
}
impl From<ClipboardError> for AppError {
    fn from(e: ClipboardError) -> Self {
        Self::Clipboard(e.to_string())
    }
}
impl From<SaveError> for AppError {
    fn from(e: SaveError) -> Self {
        Self::Save(e.to_string())
    }
}
impl From<HotkeyError> for AppError {
    fn from(e: HotkeyError) -> Self {
        Self::Hotkey(e.to_string())
    }
}
