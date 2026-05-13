use crate::capture::CaptureFrame;
use crate::config::Config;
use crate::hotkey::PlatformHotkey;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPhase {
    Idle,
    Capturing,
    Editing,
}

#[derive(Debug, Clone, Copy)]
pub enum PhaseEvent {
    HotkeyPressed,
    SelectionConfirmed,
    ActionFinished,
    Cancelled,
    ReframeRequest,
}

#[derive(thiserror::Error, Debug)]
#[error("invalid transition: {from:?} → {event:?}")]
pub struct TransitionError {
    pub from: AppPhase,
    pub event: PhaseEvent,
}

impl AppPhase {
    pub fn transition(&mut self, ev: PhaseEvent) -> Result<(), TransitionError> {
        use AppPhase::*;
        use PhaseEvent::*;
        let next = match (*self, ev) {
            (Idle, HotkeyPressed) => Capturing,
            (Capturing, SelectionConfirmed) => Editing,
            (Editing, ReframeRequest) => Capturing,
            (Editing, ActionFinished) => Idle,
            (Capturing, Cancelled) => Idle,
            (Editing, Cancelled) => Idle,
            (from, ev) => return Err(TransitionError { from, event: ev }),
        };
        *self = next;
        Ok(())
    }
}

/// Singleton runtime state, lives inside Tauri's `State`.
pub struct AppState {
    pub phase: Mutex<AppPhase>,
    pub capture: Mutex<Option<CaptureFrame>>,
    pub cropped: Mutex<Option<Vec<u8>>>,
    pub last_save_dir: Mutex<Option<PathBuf>>,
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub hotkey: Mutex<Option<PlatformHotkey>>,
    pub pins: crate::pin::registry::PinRegistry,
}

impl AppState {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        Self {
            phase: Mutex::new(AppPhase::Idle),
            capture: Mutex::new(None),
            cropped: Mutex::new(None),
            last_save_dir: Mutex::new(None),
            config: Mutex::new(config),
            config_path,
            hotkey: Mutex::new(None),
            pins: crate::pin::registry::PinRegistry::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_transitions_to_capturing() {
        let mut s = AppPhase::Idle;
        assert!(s.transition(PhaseEvent::HotkeyPressed).is_ok());
        assert_eq!(s, AppPhase::Capturing);
    }

    #[test]
    fn capturing_to_editing_on_selection() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::SelectionConfirmed).is_ok());
        assert_eq!(s, AppPhase::Editing);
    }

    #[test]
    fn editing_to_idle_on_finish() {
        let mut s = AppPhase::Editing;
        assert!(s.transition(PhaseEvent::ActionFinished).is_ok());
        assert_eq!(s, AppPhase::Idle);
    }

    #[test]
    fn hotkey_in_capturing_is_ignored() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::HotkeyPressed).is_err());
        assert_eq!(s, AppPhase::Capturing);
    }

    #[test]
    fn cancel_returns_to_idle_from_any_active_phase() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::Cancelled).is_ok());
        assert_eq!(s, AppPhase::Idle);
        let mut s = AppPhase::Editing;
        assert!(s.transition(PhaseEvent::Cancelled).is_ok());
        assert_eq!(s, AppPhase::Idle);
    }

    #[test]
    fn editing_to_capturing_on_reframe() {
        let mut s = AppPhase::Editing;
        assert!(s.transition(PhaseEvent::ReframeRequest).is_ok());
        assert_eq!(s, AppPhase::Capturing);
    }

    #[test]
    fn reframe_from_idle_is_err() {
        let mut s = AppPhase::Idle;
        assert!(s.transition(PhaseEvent::ReframeRequest).is_err());
        assert_eq!(s, AppPhase::Idle);
    }

    #[test]
    fn reframe_from_capturing_is_err() {
        let mut s = AppPhase::Capturing;
        assert!(s.transition(PhaseEvent::ReframeRequest).is_err());
        assert_eq!(s, AppPhase::Capturing);
    }
}
