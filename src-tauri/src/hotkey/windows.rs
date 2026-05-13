use super::{HotkeyError, HotkeyKind, HotkeyService};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::collections::HashMap;
use std::str::FromStr;

pub struct WindowsHotkey {
    manager: GlobalHotKeyManager,
    slots: HashMap<HotkeyKind, HotKey>,
}

// SAFETY: `GlobalHotKeyManager` holds an HWND for a message-only window. The
// `register`/`unregister` calls dispatch Win32 messages, which are safe to invoke
// from any thread. Access to the manager is already serialized through
// `Mutex<Option<WindowsHotkey>>` in `AppState`.
unsafe impl Send for WindowsHotkey {}
unsafe impl Sync for WindowsHotkey {}

impl WindowsHotkey {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self {
            manager: GlobalHotKeyManager::new()
                .map_err(|e| HotkeyError::Backend(e.to_string()))?,
            slots: HashMap::new(),
        })
    }

    pub fn event_receiver() -> crossbeam_channel::Receiver<GlobalHotKeyEvent> {
        GlobalHotKeyEvent::receiver().clone()
    }
}

impl HotkeyService for WindowsHotkey {
    fn register(&mut self, kind: HotkeyKind, combo: &str) -> Result<(), HotkeyError> {
        let hk = HotKey::from_str(combo).map_err(|_| HotkeyError::Invalid(combo.into()))?;
        if let Some(prev) = self.slots.remove(&kind) {
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
        self.slots.insert(kind, hk);
        Ok(())
    }

    fn unregister(&mut self, kind: HotkeyKind) {
        if let Some(prev) = self.slots.remove(&kind) {
            let _ = self.manager.unregister(prev);
        }
    }

    fn id_of(&self, kind: HotkeyKind) -> Option<u32> {
        self.slots.get(&kind).map(|hk| hk.id())
    }
}
