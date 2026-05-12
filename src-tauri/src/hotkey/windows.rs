use super::{HotkeyError, HotkeyService};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::str::FromStr;

pub struct WindowsHotkey {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl WindowsHotkey {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()
                .map_err(|e| HotkeyError::Backend(e.to_string()))?,
            current: None,
        })
    }

    /// Subscribe to global hotkey events. Caller polls this in a thread and
    /// dispatches into the tray-host state machine.
    pub fn event_receiver() -> crossbeam_channel::Receiver<GlobalHotKeyEvent> {
        GlobalHotKeyEvent::receiver().clone()
    }
}

impl HotkeyService for WindowsHotkey {
    fn register(&mut self, combo: &str) -> Result<(), HotkeyError> {
        let hk = HotKey::from_str(combo).map_err(|_| HotkeyError::Invalid(combo.into()))?;
        if let Some(prev) = self.current.take() {
            let _ = self.manager.unregister(prev);
        }
        self.manager.register(hk).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("registered") {
                HotkeyError::Conflict
            } else {
                HotkeyError::Backend(msg)
            }
        })?;
        self.current = Some(hk);
        Ok(())
    }

    fn unregister(&mut self) {
        if let Some(prev) = self.current.take() {
            let _ = self.manager.unregister(prev);
        }
    }
}
