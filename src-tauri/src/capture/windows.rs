use super::{Capture, CaptureError, CaptureFrame, ScreenInfo};
use screenshots::Screen;

pub struct WindowsCapture;

impl WindowsCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Capture for WindowsCapture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError> {
        let screens = Screen::all().map_err(|e| CaptureError::Backend(e.to_string()))?;
        if screens.is_empty() {
            return Err(CaptureError::Backend("no screens".into()));
        }
        let min_x = screens.iter().map(|s| s.display_info.x).min().unwrap();
        let min_y = screens.iter().map(|s| s.display_info.y).min().unwrap();
        let max_x = screens
            .iter()
            .map(|s| s.display_info.x + s.display_info.width as i32)
            .max()
            .unwrap();
        let max_y = screens
            .iter()
            .map(|s| s.display_info.y + s.display_info.height as i32)
            .max()
            .unwrap();
        let total_w = (max_x - min_x) as u32;
        let total_h = (max_y - min_y) as u32;

        let mut canvas = image::RgbaImage::from_pixel(total_w, total_h, image::Rgba([0, 0, 0, 255]));
        for s in &screens {
            let img = s
                .capture()
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            let rgba = image::RgbaImage::from_raw(
                s.display_info.width,
                s.display_info.height,
                img.into_raw(),
            )
            .ok_or_else(|| CaptureError::Backend("bad screen buffer".into()))?;
            let dx = (s.display_info.x - min_x) as i64;
            let dy = (s.display_info.y - min_y) as i64;
            image::imageops::overlay(&mut canvas, &rgba, dx, dy);
        }

        let mut png_buf = std::io::Cursor::new(Vec::new());
        canvas
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;

        let screen_infos = screens
            .iter()
            .map(|s| ScreenInfo {
                x: s.display_info.x,
                y: s.display_info.y,
                w: s.display_info.width,
                h: s.display_info.height,
                scale: s.display_info.scale_factor,
            })
            .collect();

        Ok(CaptureFrame {
            png_bytes: png_buf.into_inner(),
            width: total_w,
            height: total_h,
            origin_x: min_x,
            origin_y: min_y,
            screens: screen_infos,
        })
    }
}
