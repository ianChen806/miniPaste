use crate::clipboard::{Clipboard, PasteContent, PlatformClipboard};
use crate::pin::text_to_image;
use crate::state::AppState;
use base64::Engine;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

const MAX_PIXELS: u64 = 50_000_000;
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

pub fn paste_from_clipboard(app: &AppHandle) {
    let cb = PlatformClipboard::new();
    let content = match cb.read_paste_content() {
        Ok(c) => c,
        Err(e) => {
            emit_error(app, format!("剪貼簿讀取失敗：{}", e));
            return;
        }
    };

    let png = match content_to_png(content) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            emit_error(app, "剪貼簿是空的".into());
            return;
        }
        Err(msg) => {
            emit_error(app, msg);
            return;
        }
    };

    if let Err(msg) = spawn_pin(app, png) {
        emit_error(app, msg);
    }
}

fn content_to_png(content: PasteContent) -> Result<Option<Vec<u8>>, String> {
    match content {
        PasteContent::Empty => Ok(None),
        PasteContent::Image(bytes) => {
            let img = image::load_from_memory(&bytes)
                .map_err(|e| format!("圖片格式無法解析：{}", e))?;
            let mut out = std::io::Cursor::new(Vec::new());
            img.write_to(&mut out, image::ImageFormat::Png)
                .map_err(|e| format!("PNG 編碼失敗：{}", e))?;
            Ok(Some(out.into_inner()))
        }
        PasteContent::Text(s) => match text_to_image::render(&s) {
            Ok(png) => Ok(Some(png)),
            Err(text_to_image::TextRenderError::Empty) => Ok(None),
            Err(e) => Err(format!("文字渲染失敗：{}", e)),
        },
        PasteContent::FilePath(p) => path_to_png(&p).map(Some),
    }
}

fn path_to_png(path: &Path) -> Result<Vec<u8>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !IMAGE_EXTS.iter().any(|e| *e == ext) {
        return Err(format!(
            "不是圖片：{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| format!("找不到檔案：{}（{}）", path.display(), e))?;
    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("圖片格式無法解析：{}", e))?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("PNG 編碼失敗：{}", e))?;
    Ok(out.into_inner())
}

fn spawn_pin(app: &AppHandle, png_bytes: Vec<u8>) -> Result<(), String> {
    let img =
        image::load_from_memory(&png_bytes).map_err(|e| format!("圖片解碼失敗：{}", e))?;
    let (w, h) = (img.width(), img.height());
    if (w as u64) * (h as u64) > MAX_PIXELS {
        return Err("內容過大".into());
    }

    let (target_w, target_h) = clamp_to_screen(app, w, h);

    let state: tauri::State<AppState> = app.state();
    let label = state
        .pins
        .reserve()
        .map_err(|_| "Pin 上限 30 個".to_string())?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    let (px, py) = match app.cursor_position() {
        Ok(p) => (p.x as f64, p.y as f64),
        Err(_) => (200.0, 200.0),
    };

    let init_script = format!(
        "window.__pinData = {{ label: \"{label}\", image_b64: \"{b64}\", width: {w}, height: {h} }};"
    );

    let build_result = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("pin.html".into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .visible(false)
    .inner_size(target_w, target_h)
    .position(px, py)
    .initialization_script(&init_script)
    .build();

    match build_result {
        Ok(win) => {
            let app_for_event = app.clone();
            let label_for_event = label.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let state: tauri::State<AppState> = app_for_event.state();
                    state.pins.release(&label_for_event);
                    tracing::info!("pin '{}' close requested", label_for_event);
                }
            });
            let _ = win.show();
            let _ = win.set_focus();
            tracing::info!("pin spawned: {} ({}x{})", label, target_w, target_h);
            Ok(())
        }
        Err(e) => {
            state.pins.release(&label);
            tracing::error!("pin build failed: {}", e);
            Err("無法建立視窗".into())
        }
    }
}

fn clamp_to_screen(app: &AppHandle, w: u32, h: u32) -> (f64, f64) {
    let monitor = app.primary_monitor().ok().flatten();
    let (mw, mh) = match monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            (
                (size.width as f64) / scale * 0.8,
                (size.height as f64) / scale * 0.8,
            )
        }
        None => (1280.0, 720.0),
    };
    let aspect = (w as f64) / (h as f64).max(1.0);
    let mut tw = w as f64;
    let mut th = h as f64;
    if tw > mw {
        tw = mw;
        th = tw / aspect;
    }
    if th > mh {
        th = mh;
        tw = th * aspect;
    }
    (tw.max(40.0), th.max(40.0))
}

fn emit_error(app: &AppHandle, reason: String) {
    tracing::warn!("pin-error: {}", reason);
    let _ = app.emit("pin-error", serde_json::json!({ "reason": reason }));
}
