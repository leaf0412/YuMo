use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
    menu::{Menu, MenuItem},
};

use crate::window_manager::WindowManager;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = Menu::with_items(app, &[
        &MenuItem::with_id(app, "show", "打开 VoiceInk", true, None::<&str>)?,
        &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
    ])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            let wm = WindowManager::new(app.clone());
            match event.id.as_ref() {
                "show" => wm.show("main"),
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let wm = WindowManager::new(tray.app_handle().clone());
                wm.show("main");
            }
        })
        .build(app)?;

    Ok(())
}
