use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Pure data types (testable without Tauri runtime)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl WindowPosition {
    /// Default recorder position: centered horizontally, just below menu bar.
    pub fn default_recorder(screen_width: u32, _screen_height: u32) -> Self {
        let width = 200.0;
        let height = 200.0;
        let x = (screen_width as f64 - width) / 2.0;
        let y = 25.0; // macOS menu bar height
        Self { x, y, width, height }
    }

    /// Clamp position so the window stays within screen bounds.
    pub fn clamp_to_screen(&self, screen_width: f64, screen_height: f64) -> Self {
        let x = self.x.max(0.0).min(screen_width - self.width);
        let y = self.y.max(0.0).min(screen_height - self.height);
        Self { x, y, width: self.width, height: self.height }
    }
}

/// Stores positions for multiple windows by label. Serializable to/from DB.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowLayout {
    positions: HashMap<String, WindowPosition>,
}

impl WindowLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_position(&self, label: &str) -> Option<&WindowPosition> {
        self.positions.get(label)
    }

    pub fn set_position(&mut self, label: &str, pos: WindowPosition) {
        self.positions.insert(label.to_string(), pos);
    }
}

// ---------------------------------------------------------------------------
// Tauri integration (requires runtime, not unit-testable)
// ---------------------------------------------------------------------------

use tauri::{AppHandle, Manager};

/// Manages window lifecycle, position persistence, and visibility.
pub struct WindowManager {
    app: AppHandle,
}

impl WindowManager {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Show a window by label. Restores saved position from DB if available.
    pub fn show(&self, label: &str) {
        let Some(win) = self.app.get_webview_window(label) else {
            info!("[wm] window '{}' not found", label);
            return;
        };

        // Restore position from DB, or compute default for recorder
        if let Some(pos) = self.load_position(label) {
            let _ = win.set_position(tauri::LogicalPosition::new(pos.x, pos.y));
            let _ = win.set_size(tauri::LogicalSize::new(pos.width, pos.height));
            info!("[wm] '{}' restored to ({}, {})", label, pos.x, pos.y);
        } else if label == "recorder" {
            // First time: center below menu bar
            if let Ok(monitor) = win.current_monitor() {
                if let Some(m) = monitor {
                    let s = m.size();
                    let scale = m.scale_factor();
                    let sw = s.width as f64 / scale;
                    let pos = WindowPosition::default_recorder(sw as u32, 0);
                    let _ = win.set_position(tauri::LogicalPosition::new(pos.x, pos.y));
                    info!("[wm] '{}' default position ({}, {})", label, pos.x, pos.y);
                }
            }
        }

        let _ = win.show();

        // Only focus non-overlay windows (recorder should not steal focus)
        if label != "recorder" {
            let _ = win.set_focus();
        }
        info!("[wm] '{}' shown", label);
    }

    /// Hide a window by label. Saves its current position to DB.
    pub fn hide(&self, label: &str) {
        let Some(win) = self.app.get_webview_window(label) else { return; };

        // Save logical position before hiding (divide physical by scale factor)
        let scale = win.scale_factor().unwrap_or(1.0);
        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) {
            let wp = WindowPosition {
                x: pos.x as f64 / scale,
                y: pos.y as f64 / scale,
                width: size.width as f64 / scale,
                height: size.height as f64 / scale,
            };
            self.save_position(label, &wp);
        }

        // Check if main window is visible before hiding
        let main_was_visible = self.app.get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);

        let _ = win.hide();
        info!("[wm] '{}' hidden", label);

        // If main window wasn't visible before, hide the app to prevent
        // macOS from auto-focusing it when recorder disappears
        if label == "recorder" && !main_was_visible {
            #[cfg(target_os = "macos")]
            {
                use objc::{msg_send, sel, sel_impl};
                unsafe {
                    let app: cocoa::base::id = msg_send![
                        objc::runtime::Class::get("NSApplication").unwrap(),
                        sharedApplication
                    ];
                    let _: () = msg_send![app, hide: cocoa::base::nil];
                }
                info!("[wm] app hidden to prevent main window focus");
            }
        }
    }

    /// Toggle visibility of a window.
    pub fn toggle(&self, label: &str) {
        let Some(win) = self.app.get_webview_window(label) else { return; };
        if win.is_visible().unwrap_or(false) {
            self.hide(label);
        } else {
            self.show(label);
        }
    }

    // -----------------------------------------------------------------------
    // DB persistence — stores all window positions as a single JSON setting
    // -----------------------------------------------------------------------

    fn load_layout(&self) -> WindowLayout {
        let state = match self.app.try_state::<crate::state::AppContext>() {
            Some(s) => s,
            None => return WindowLayout::new(),
        };
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => return WindowLayout::new(),
        };
        let settings = match crate::db::get_all_settings(&conn) {
            Ok(s) => s,
            Err(_) => return WindowLayout::new(),
        };
        settings
            .get("window_layout")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    fn save_layout(&self, layout: &WindowLayout) {
        let state = match self.app.try_state::<crate::state::AppContext>() {
            Some(s) => s,
            None => return,
        };
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Ok(json) = serde_json::to_string(layout) {
            let _ = crate::db::update_setting(&conn, "window_layout", &serde_json::Value::String(json));
        }
    }

    fn load_position(&self, label: &str) -> Option<WindowPosition> {
        self.load_layout().get_position(label).cloned()
    }

    fn save_position(&self, label: &str, pos: &WindowPosition) {
        let mut layout = self.load_layout();
        layout.set_position(label, pos.clone());
        self.save_layout(&layout);
        info!("[wm] saved position for '{}': ({}, {})", label, pos.x, pos.y);
    }
}
