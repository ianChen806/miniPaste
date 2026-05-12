use super::{Clipboard, ClipboardError};
use std::path::PathBuf;

pub struct MacosClipboard;

impl MacosClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for MacosClipboard {
    fn write_image(&self, _png_bytes: &[u8]) -> Result<(), ClipboardError> {
        unimplemented!("non-Windows clipboard deferred")
    }

    fn write_file_paths(&self, _paths: &[PathBuf]) -> Result<(), ClipboardError> {
        unimplemented!("non-Windows clipboard deferred")
    }
}
