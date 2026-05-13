use crate::hotkey::{HotkeyKind, HotkeyService};
use crate::state::{AppState, PhaseEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Spawn a background thread that listens for hotkey events and dispatches.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let rx = global_hotkey::GlobalHotKeyEvent::receiver();
        while let Ok(event) = rx.recv() {
            if event.state == global_hotkey::HotKeyState::Released {
                continue;
            }
            dispatch(&app, event.id);
        }
    });
}

fn dispatch(app: &AppHandle, event_id: u32) {
    let state: tauri::State<AppState> = app.state();
    let kind_opt = {
        let hk_slot = state.hotkey.lock().unwrap();
        hk_slot.as_ref().and_then(|hk| {
            if hk.id_of(HotkeyKind::Capture) == Some(event_id) {
                Some(HotkeyKind::Capture)
            } else if hk.id_of(HotkeyKind::PastePin) == Some(event_id) {
                Some(HotkeyKind::PastePin)
            } else {
                None
            }
        })
    };
    let Some(kind) = kind_opt else { return };

    match kind {
        HotkeyKind::Capture => {
            let mut phase = state.phase.lock().unwrap();
            if phase.transition(PhaseEvent::HotkeyPressed).is_err() {
                return;
            }
            drop(phase);
            let _ = app.emit("tray://trigger-capture", ());
        }
        HotkeyKind::PastePin => {
            crate::pin::service::paste_from_clipboard(app);
        }
    }
}
