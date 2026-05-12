use super::{Clipboard, ClipboardError};
use std::path::PathBuf;

pub struct WindowsClipboard;

impl WindowsClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for WindowsClipboard {
    fn write_image(&self, _png_bytes: &[u8]) -> Result<(), ClipboardError> {
        Err(ClipboardError::Backend("not yet implemented".into()))
    }

    fn write_file_paths(&self, _paths: &[PathBuf]) -> Result<(), ClipboardError> {
        Err(ClipboardError::Backend("not yet implemented".into()))
    }
}
