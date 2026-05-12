#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod macos;

pub mod trigger;

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

    /// Default impl delegates to the platform-agnostic `crop_png` helper.
    fn crop(&self, frame: &CaptureFrame, rect: Rect) -> Result<Vec<u8>, CaptureError> {
        crop_png(frame, rect)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("capture backend failed: {0}")]
    Backend(String),
    #[error("rect outside frame")]
    OutOfBounds,
}

/// Crop a region from the encoded virtual-desktop frame and return PNG bytes.
pub fn crop_png(frame: &CaptureFrame, rect: Rect) -> Result<Vec<u8>, CaptureError> {
    let img = image::load_from_memory(&frame.png_bytes)
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    let lx = (rect.x - frame.origin_x) as u32;
    let ly = (rect.y - frame.origin_y) as u32;
    if lx + rect.w > frame.width || ly + rect.h > frame.height {
        return Err(CaptureError::OutOfBounds);
    }
    let cropped = img.crop_imm(lx, ly, rect.w, rect.h);
    let mut buf = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(w: u32, h: u32) -> CaptureFrame {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        CaptureFrame {
            png_bytes: buf.into_inner(),
            width: w,
            height: h,
            origin_x: 0,
            origin_y: 0,
            screens: vec![ScreenInfo {
                x: 0,
                y: 0,
                w,
                h,
                scale: 1.0,
            }],
        }
    }

    #[test]
    fn crop_clips_a_rect_and_returns_png() {
        let frame = make_frame(4, 4);
        let cropped = crop_png(&frame, Rect { x: 1, y: 1, w: 2, h: 2 }).unwrap();
        let decoded = image::load_from_memory(&cropped).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
    }

    #[test]
    fn crop_out_of_bounds_errors() {
        let frame = make_frame(2, 2);
        assert!(matches!(
            crop_png(&frame, Rect { x: 1, y: 1, w: 5, h: 5 }),
            Err(CaptureError::OutOfBounds)
        ));
    }
}
