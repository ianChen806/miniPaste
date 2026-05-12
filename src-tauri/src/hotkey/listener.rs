use crate::state::{AppState, PhaseEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Spawn a background thread that listens for hotkey events and dispatches.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let rx = global_hotkey::GlobalHotKeyEvent::receiver();
        while let Ok(_event) = rx.recv() {
            // Only one hotkey is registered at a time so any event = trigger.
            handle_hotkey(&app);
        }
    });
}

fn handle_hotkey(app: &AppHandle) {
    let state: tauri::State<AppState> = app.state();
    let mut phase = state.phase.lock().unwrap();
    if phase.transition(PhaseEvent::HotkeyPressed).is_err() {
        return; // not idle, ignore
    }
    drop(phase);
    let _ = app.emit("tray://trigger-capture", ());
}
