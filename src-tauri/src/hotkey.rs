use log::{error, info};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Register a global shortcut that fires `callback` on key-down.
pub fn register_shortcut(
    app: &AppHandle,
    shortcut_str: &str,
    callback: impl Fn() + Send + Sync + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("[hotkey] registering shortcut: {}", shortcut_str);
    app.global_shortcut().on_shortcut(shortcut_str, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            callback();
        }
    }).map_err(|e| {
        error!("[hotkey] failed to register shortcut {}: {}", shortcut_str, e);
        e
    })?;

    info!("[hotkey] shortcut registered: {}", shortcut_str);
    Ok(())
}

/// Remove all registered global shortcuts.
pub fn unregister_all(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    info!("[hotkey] unregistering all shortcuts");
    app.global_shortcut().unregister_all().map_err(|e| {
        error!("[hotkey] failed to unregister all shortcuts: {}", e);
        e
    })?;
    info!("[hotkey] all shortcuts unregistered");
    Ok(())
}
