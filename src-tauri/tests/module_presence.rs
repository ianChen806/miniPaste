#[test]
fn platform_modules_export_aliases() {
    // Smoke test: each platform module re-exports a platform-specific
    // implementation type behind a `PlatformX` alias.
    use minipaste::{capture, clipboard, hotkey, tray};
    let _: Option<hotkey::PlatformHotkey> = None;
    let _: Option<capture::PlatformCapture> = None;
    let _: Option<clipboard::PlatformClipboard> = None;
    let _: Option<tray::PlatformTray> = None;
}
