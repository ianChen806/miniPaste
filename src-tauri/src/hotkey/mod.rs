// On non-Windows targets we fall back to the macos.rs stub (which contains
// `unimplemented!()`). This lets the project compile during Linux dev while
// keeping the Windows-only production path intact.
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

pub mod listener;

#[cfg(target_os = "windows")]
pub use windows::WindowsHotkey as PlatformHotkey;
#[cfg(not(target_os = "windows"))]
pub use macos::MacosHotkey as PlatformHotkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyKind {
    Capture,
    PastePin,
}

pub trait HotkeyService: Send + Sync {
    fn register(&mut self, kind: HotkeyKind, combo: &str) -> Result<(), HotkeyError>;
    fn unregister(&mut self, kind: HotkeyKind);
    fn id_of(&self, kind: HotkeyKind) -> Option<u32>;
}

#[derive(thiserror::Error, Debug)]
pub enum HotkeyError {
    #[error("invalid hotkey combo: {0}")]
    Invalid(String),
    #[error("hotkey already in use")]
    Conflict,
    #[error("backend error: {0}")]
    Backend(String),
}
