use super::{HotkeyError, HotkeyService};

pub struct WindowsHotkey;

impl WindowsHotkey {
    pub fn new() -> Self {
        Self
    }
}

impl HotkeyService for WindowsHotkey {
    fn register(&mut self, _combo: &str) -> Result<(), HotkeyError> {
        Ok(())
    }

    fn unregister(&mut self) {}
}
