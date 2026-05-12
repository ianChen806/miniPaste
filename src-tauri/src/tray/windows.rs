use super::{TrayEvent, TrayService};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub struct WindowsTray;

impl WindowsTray {
    pub fn new() -> Self {
        Self
    }
}

impl TrayService for WindowsTray {
    fn build(&self, app: &AppHandle) -> tauri::Result<()> {
        build_tray(app)
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItemBuilder::with_id("capture", "Capture").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app)
        .item(&capture)
        .item(&settings)
        .item(&separator)
        .item(&quit)
        .build()?;

    let _tray = TrayIconBuilder::with_id("minipaste-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| match event.id.as_ref() {
            "capture" => dispatch(app_handle, TrayEvent::TriggerCapture),
            "settings" => dispatch(app_handle, TrayEvent::OpenSettings),
            "quit" => dispatch(app_handle, TrayEvent::Quit),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                dispatch(tray.app_handle(), TrayEvent::OpenSettings);
            }
        })
        .build(app)?;
    Ok(())
}

fn dispatch(app: &AppHandle, ev: TrayEvent) {
    match ev {
        TrayEvent::OpenSettings => {
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        TrayEvent::TriggerCapture => {
            let _ = app.emit("tray://trigger-capture", ());
        }
        TrayEvent::Quit => app.exit(0),
    }
}
