use super::{Capture, CaptureError, CaptureFrame, ScreenInfo};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use screenshots::Screen;

pub struct WindowsCapture;

impl WindowsCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Capture for WindowsCapture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError> {
        let t0 = std::time::Instant::now();
        let screens = Screen::all().map_err(|e| CaptureError::Backend(e.to_string()))?;
        if screens.is_empty() {
            return Err(CaptureError::Backend("no screens".into()));
        }
        let t_enum = t0.elapsed();
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
        let t_canvas = t0.elapsed();
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
        let t_capture_compose = t0.elapsed();

        // Sub filter: single-pass prediction (current pixel - left pixel). Cheap
        // (one subtract per pixel, no try-all-5-variants like Adaptive) and still
        // gives zlib a low-entropy stream — PNG stays a few MB, encode time stays
        // bounded. Adaptive's per-row 5-filter sweep is too expensive on 30 MB RGBA.
        let mut png_buf = std::io::Cursor::new(Vec::new());
        let encoder = PngEncoder::new_with_quality(
            &mut png_buf,
            CompressionType::Fast,
            FilterType::Sub,
        );
        encoder
            .write_image(canvas.as_raw(), total_w, total_h, ExtendedColorType::Rgba8)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let t_encode = t0.elapsed();

        tracing::info!(
            "virtual_desktop timing: enum={:?} canvas_alloc={:?} capture+compose={:?} png_encode={:?} total={:?}, png_bytes={}, raw_bytes={}",
            t_enum,
            t_canvas - t_enum,
            t_capture_compose - t_canvas,
            t_encode - t_capture_compose,
            t_encode,
            png_buf.get_ref().len(),
            canvas.as_raw().len()
        );

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
