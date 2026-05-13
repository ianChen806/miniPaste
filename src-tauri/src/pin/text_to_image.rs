use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

const FONT_SIZE: f32 = 16.0;
const PADDING: u32 = 12;
const BG: [u8; 4] = [0x1f, 0x1f, 0x1f, 0xff];
const FG: [u8; 4] = [0xe5, 0xe5, 0xe5, 0xff];
const MAX_LINES: usize = 200;
const MAX_LINE_CHARS: usize = 200;
const TRUNCATE_SUFFIX: &str = "\u{22ef}（截斷）";

#[derive(thiserror::Error, Debug)]
pub enum TextRenderError {
    #[error("font load failed: {0}")]
    Font(String),
    #[error("image encode failed: {0}")]
    Encode(String),
    #[error("empty text after trim")]
    Empty,
}

/// Render the given text to a PNG byte stream.
///
/// Font strategy:
/// - On Windows, try `msyh.ttc` (Microsoft YaHei) for CJK + Latin coverage,
///   then fall back to Segoe UI.
/// - On other platforms, the font loader returns an error in MVP.
pub fn render(text: &str) -> Result<Vec<u8>, TextRenderError> {
    if text.trim().is_empty() {
        return Err(TextRenderError::Empty);
    }

    let font = load_font()?;
    let scale = PxScale::from(FONT_SIZE);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_h = (ascent - descent + scaled.line_gap()).ceil() as u32;

    let (lines, capped) = wrap_lines(text);
    let visible: Vec<String> = if capped {
        let mut v = lines;
        if let Some(last) = v.last_mut() {
            last.push_str(TRUNCATE_SUFFIX);
        }
        v
    } else {
        lines
    };

    let mut max_w_px = 0_f32;
    for line in &visible {
        let mut w = 0.0;
        let mut last: Option<ab_glyph::GlyphId> = None;
        for ch in line.chars() {
            let gid = font.glyph_id(ch);
            if let Some(prev) = last {
                w += scaled.kern(prev, gid);
            }
            w += scaled.h_advance(gid);
            last = Some(gid);
        }
        if w > max_w_px {
            max_w_px = w;
        }
    }
    let img_w = (max_w_px.ceil() as u32).max(1) + PADDING * 2;
    let img_h = (visible.len() as u32).max(1) * line_h + PADDING * 2;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(img_w, img_h, Rgba(BG));

    for (row, line) in visible.iter().enumerate() {
        let baseline_y = PADDING as f32 + (row as f32) * (line_h as f32) + ascent;
        let mut pen_x = PADDING as f32;
        let mut last: Option<ab_glyph::GlyphId> = None;

        for ch in line.chars() {
            let gid = font.glyph_id(ch);
            if let Some(prev) = last {
                pen_x += scaled.kern(prev, gid);
            }
            let glyph = gid.with_scale_and_position(scale, ab_glyph::point(pen_x, baseline_y));
            if let Some(outline) = font.outline_glyph(glyph) {
                let bb = outline.px_bounds();
                outline.draw(|gx, gy, cov| {
                    let px = bb.min.x as i32 + gx as i32;
                    let py = bb.min.y as i32 + gy as i32;
                    if px < 0 || py < 0 || px as u32 >= img_w || py as u32 >= img_h {
                        return;
                    }
                    let base = img.get_pixel(px as u32, py as u32).0;
                    img.put_pixel(px as u32, py as u32, Rgba(blend(base, FG, cov)));
                });
            }
            pen_x += scaled.h_advance(gid);
            last = Some(gid);
        }
    }

    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| TextRenderError::Encode(e.to_string()))?;
    Ok(out.into_inner())
}

fn wrap_lines(text: &str) -> (Vec<String>, bool) {
    let mut out: Vec<String> = Vec::new();
    let mut capped = false;
    for line in text.lines() {
        if out.len() >= MAX_LINES {
            capped = true;
            break;
        }
        if line.chars().count() <= MAX_LINE_CHARS {
            out.push(line.to_string());
        } else {
            let chars: Vec<char> = line.chars().collect();
            for chunk in chars.chunks(MAX_LINE_CHARS) {
                if out.len() >= MAX_LINES {
                    capped = true;
                    break;
                }
                out.push(chunk.iter().collect());
            }
        }
    }
    (out, capped)
}

fn blend(base: [u8; 4], fg: [u8; 4], cov: f32) -> [u8; 4] {
    let a = cov.clamp(0.0, 1.0);
    let mix = |b: u8, f: u8| -> u8 { ((b as f32) * (1.0 - a) + (f as f32) * a).round() as u8 };
    [
        mix(base[0], fg[0]),
        mix(base[1], fg[1]),
        mix(base[2], fg[2]),
        0xff,
    ]
}

#[cfg(target_os = "windows")]
fn load_font() -> Result<FontArc, TextRenderError> {
    use std::fs;
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
    ];
    for p in candidates {
        if let Ok(bytes) = fs::read(p) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Ok(font);
            }
        }
    }
    Err(TextRenderError::Font(
        "no usable Windows font found (tried msyh, segoeui)".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn load_font() -> Result<FontArc, TextRenderError> {
    Err(TextRenderError::Font(
        "paste-pin font loader is Windows-only in MVP".into(),
    ))
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn render_ascii_returns_decodable_png() {
        let png = render("hello world").expect("render");
        assert!(!png.is_empty());
        let img = image::load_from_memory(&png).expect("decode");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn render_cjk_returns_decodable_png() {
        let png = render("你好，世界").expect("render");
        let img = image::load_from_memory(&png).expect("decode");
        assert!(img.width() > 0);
    }

    #[test]
    fn render_empty_errors() {
        assert!(matches!(render("   \n   "), Err(TextRenderError::Empty)));
    }

    #[test]
    fn render_truncates_when_over_max_lines() {
        let many = (0..(MAX_LINES + 10))
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let png = render(&many).expect("render");
        let img = image::load_from_memory(&png).expect("decode");
        let line_count_visible = (img.height() - 2 * PADDING) / 19;
        assert!(line_count_visible <= MAX_LINES as u32);
    }
}
