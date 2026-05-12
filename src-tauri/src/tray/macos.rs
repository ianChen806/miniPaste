use super::TrayService;
use tauri::AppHandle;

pub struct MacosTray;

impl MacosTray {
    pub fn new() -> Self {
        Self
    }
}

impl TrayService for MacosTray {
    fn build(&self, app: &AppHandle) -> tauri::Result<()> {
        build_tray(app)
    }
}

/// Non-Windows fallback. Used for both macOS (deferred real impl) and Linux
/// (dev). On dev we register no tray; the hotkey IPC path still works.
pub fn build_tray(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}
