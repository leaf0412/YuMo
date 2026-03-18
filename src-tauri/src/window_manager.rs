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
    /// Default recorder position: centered horizontally, near the top.
    pub fn default_recorder(screen_width: u32, screen_height: u32) -> Self {
        let width = 200.0;
        let height = 200.0;
        let x = (screen_width as f64 - width) / 2.0;
        let y = 30.0;
        let _ = screen_height; // used for future multi-monitor
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

        // Restore position from DB
        if let Some(pos) = self.load_position(label) {
            let _ = win.set_position(tauri::PhysicalPosition::new(pos.x as i32, pos.y as i32));
            let _ = win.set_size(tauri::PhysicalSize::new(pos.width as u32, pos.height as u32));
            info!("[wm] '{}' restored to ({}, {})", label, pos.x, pos.y);
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

        // Save position before hiding
        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) {
            let wp = WindowPosition {
                x: pos.x as f64,
                y: pos.y as f64,
                width: size.width as f64,
                height: size.height as f64,
            };
            self.save_position(label, &wp);
        }

        let _ = win.hide();
        info!("[wm] '{}' hidden", label);
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
        let state = match self.app.try_state::<crate::state::AppState>() {
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
        let state = match self.app.try_state::<crate::state::AppState>() {
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
