// Non-Windows fallback for the hotkey service.
// On macOS this is where the real Carbon / event-tap impl will live (deferred).
// On Linux (dev) this remains an `unimplemented!()` stub.
use super::{HotkeyError, HotkeyService};

pub struct MacosHotkey;

impl MacosHotkey {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self)
    }
}

impl HotkeyService for MacosHotkey {
    fn register(&mut self, _combo: &str) -> Result<(), HotkeyError> {
        unimplemented!("non-Windows hotkey support deferred")
    }

    fn unregister(&mut self) {}
}
