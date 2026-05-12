#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::WindowsCapture as PlatformCapture;
#[cfg(not(target_os = "windows"))]
pub use macos::MacosCapture as PlatformCapture;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenInfo {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub screens: Vec<ScreenInfo>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub trait Capture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError>;
    fn crop(&self, frame: &CaptureFrame, rect: Rect) -> Result<Vec<u8>, CaptureError>;
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("capture backend failed: {0}")]
    Backend(String),
    #[error("rect outside frame")]
    OutOfBounds,
}
