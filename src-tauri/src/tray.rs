use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
    menu::{Menu, MenuItem},
};
use log::info;

use crate::window_manager::WindowManager;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = Menu::with_items(app, &[
        &MenuItem::with_id(app, "show", "打开语墨", true, None::<&str>)?,
        &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
    ])?;
    info!("[tray] [setup] menu initialized");

    let _tray = TrayIconBuilder::new()
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon@2x.png")).unwrap())
        .menu(&menu)
        .on_menu_event(|app, event| {
            info!("[tray] [menu_event] action={}", event.id.as_ref());
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
                info!("[tray] [icon_click]");
                let wm = WindowManager::new(tray.app_handle().clone());
                wm.show("main");
            }
        })
        .build(app)?;

    Ok(())
}
