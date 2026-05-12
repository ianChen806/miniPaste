use super::TrayService;

pub struct WindowsTray;

impl WindowsTray {
    pub fn new() -> Self {
        Self
    }
}

impl TrayService for WindowsTray {
    fn build(&self, app: &tauri::AppHandle) -> tauri::Result<()> {
        build_tray(app)
    }
}

pub fn build_tray(_app: &tauri::AppHandle) -> tauri::Result<()> {
    // Real impl arrives in Task 17.
    Ok(())
}
