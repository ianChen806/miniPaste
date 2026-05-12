use crate::capture::{Capture, PlatformCapture, ScreenInfo};
use crate::state::AppState;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[derive(Serialize, Clone)]
pub struct CaptureReadyPayload {
    pub thumbnail_b64: String,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub screens: Vec<ScreenInfo>,
}

pub fn trigger_capture(app: &AppHandle) -> Result<(), String> {
    tracing::info!("trigger_capture invoked");
    let cap = PlatformCapture::new();
    let frame = cap.virtual_desktop().map_err(|e| e.to_string())?;
    tracing::info!(
        "trigger_capture: frame {}x{} origin=({},{}), {} screens",
        frame.width,
        frame.height,
        frame.origin_x,
        frame.origin_y,
        frame.screens.len()
    );

    // Store full-res frame in state for later crop.
    let state: tauri::State<AppState> = app.state();
    *state.capture.lock().unwrap() = Some(frame.clone());

    let b64 = base64::engine::general_purpose::STANDARD.encode(&frame.png_bytes);
    let payload = CaptureReadyPayload {
        thumbnail_b64: b64,
        width: frame.width,
        height: frame.height,
        origin_x: frame.origin_x,
        origin_y: frame.origin_y,
        screens: frame.screens.clone(),
    };

    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.set_position(PhysicalPosition {
            x: frame.origin_x,
            y: frame.origin_y,
        });
        let _ = win.set_size(PhysicalSize {
            width: frame.width,
            height: frame.height,
        });
        let _ = win.set_always_on_top(true);
        let r_show = win.show();
        let r_focus = win.set_focus();
        let r_emit = win.emit("capture-ready", payload);
        tracing::info!(
            "trigger_capture: overlay show={:?} focus={:?} emit={:?}",
            r_show,
            r_focus,
            r_emit
        );
    } else {
        tracing::warn!("trigger_capture: overlay window NOT FOUND");
    }
    Ok(())
}
