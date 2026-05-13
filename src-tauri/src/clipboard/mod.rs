#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::WindowsClipboard as PlatformClipboard;
#[cfg(not(target_os = "windows"))]
pub use macos::MacosClipboard as PlatformClipboard;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteContent {
    Image(Vec<u8>),
    Text(String),
    FilePath(PathBuf),
    Empty,
}

pub trait Clipboard {
    fn write_image(&self, png_bytes: &[u8]) -> Result<(), ClipboardError>;
    fn write_file_paths(&self, paths: &[PathBuf]) -> Result<(), ClipboardError>;
    fn read_paste_content(&self) -> Result<PasteContent, ClipboardError>;
}

#[derive(thiserror::Error, Debug)]
pub enum ClipboardError {
    #[error("clipboard busy after retries")]
    Busy,
    #[error("backend error: {0}")]
    Backend(String),
}
