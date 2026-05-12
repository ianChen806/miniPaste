#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::{build_tray, WindowsTray as PlatformTray};
#[cfg(not(target_os = "windows"))]
pub use macos::{build_tray, MacosTray as PlatformTray};

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    OpenSettings,
    TriggerCapture,
    Quit,
}

pub trait TrayService {
    fn build(&self, app: &tauri::AppHandle) -> tauri::Result<()>;
}
