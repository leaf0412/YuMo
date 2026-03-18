use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Register a global shortcut that fires `callback` on key-down.
pub fn register_shortcut(
    app: &AppHandle,
    shortcut_str: &str,
    callback: impl Fn() + Send + Sync + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().on_shortcut(shortcut_str, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            callback();
        }
    })?;

    Ok(())
}

/// Remove all registered global shortcuts.
pub fn unregister_all(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().unregister_all()?;
    Ok(())
}
