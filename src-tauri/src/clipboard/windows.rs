use super::{Clipboard, ClipboardError};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

pub struct WindowsClipboard;

impl WindowsClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for WindowsClipboard {
    fn write_image(&self, png_bytes: &[u8]) -> Result<(), ClipboardError> {
        // arboard expects raw RGBA, so decode the PNG first.
        let img = image::load_from_memory(png_bytes)
            .map_err(|e| ClipboardError::Backend(e.to_string()))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        let raw = img.into_raw();
        retry_3(|| {
            let data = arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: std::borrow::Cow::Borrowed(&raw),
            };
            let mut cb = arboard::Clipboard::new()
                .map_err(|e| ClipboardError::Backend(e.to_string()))?;
            cb.set_image(data)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        })
    }

    fn write_file_paths(&self, paths: &[PathBuf]) -> Result<(), ClipboardError> {
        let strings: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        retry_3(|| {
            clipboard_win::set_clipboard(clipboard_win::formats::FileList, &strings)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        })
    }
}

fn retry_3<F>(mut f: F) -> Result<(), ClipboardError>
where
    F: FnMut() -> Result<(), ClipboardError>,
{
    for attempt in 0..3 {
        match f() {
            Ok(()) => return Ok(()),
            Err(_) if attempt < 2 => sleep(Duration::from_millis(50)),
            Err(e) => return Err(e),
        }
    }
    Err(ClipboardError::Busy)
}
