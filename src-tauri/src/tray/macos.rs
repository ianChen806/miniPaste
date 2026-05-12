use super::TrayService;

pub struct MacosTray;

impl MacosTray {
    pub fn new() -> Self {
        Self
    }
}

impl TrayService for MacosTray {
    fn build(&self, _app: &tauri::AppHandle) -> tauri::Result<()> {
        unimplemented!("non-Windows tray deferred")
    }
}

pub fn build_tray(_app: &tauri::AppHandle) -> tauri::Result<()> {
    // Non-Windows fallback (dev/macOS). Real impl deferred.
    Ok(())
}
