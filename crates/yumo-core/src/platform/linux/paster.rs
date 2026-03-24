use arboard::Clipboard;

use crate::error::AppResult;
use crate::platform::traits::PlatformPaster;

// ---------------------------------------------------------------------------
// LinuxPaster — PlatformPaster implementation
//
// Linux strategy: write text to clipboard only, user pastes with Ctrl+V.
// No paste simulation — xdotool/wtype/enigo all have reliability issues
// across different desktop environments and Wayland/X11 combinations.
// ---------------------------------------------------------------------------

pub struct LinuxPaster;

impl PlatformPaster for LinuxPaster {
    fn read_clipboard() -> AppResult<Option<String>> {
        Ok(read_clipboard_impl())
    }

    fn write_clipboard(text: &str) -> AppResult<()> {
        write_clipboard_impl(text);
        Ok(())
    }

    fn save_clipboard() -> AppResult<Option<String>> {
        Ok(read_clipboard_impl())
    }

    fn restore_clipboard(saved: Option<String>) -> AppResult<()> {
        if let Some(text) = saved {
            write_clipboard_impl(&text);
        }
        Ok(())
    }

    fn simulate_paste() -> AppResult<()> {
        Ok(()) // no-op on Linux
    }
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

pub fn read_clipboard() -> Option<String> {
    read_clipboard_impl()
}

pub fn write_clipboard(text: &str) {
    write_clipboard_impl(text);
}

pub fn save_clipboard() -> Option<String> {
    read_clipboard_impl()
}

pub fn restore_clipboard(saved: Option<String>) {
    if let Some(text) = saved {
        write_clipboard_impl(&text);
    }
}

pub fn simulate_paste() {
    // no-op on Linux
}

/// On Linux, paste_text is a no-op.
/// Clipboard write is handled by the Electron/Tauri layer (Chromium clipboard API)
/// because arboard's X11 clipboard loses content when the Clipboard object is dropped.
pub fn paste_text(_text: &str, _restore_delay_ms: u64) -> AppResult<()> {
    log::info!("[paster] Linux: skipping Rust clipboard write, delegated to frontend");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

fn read_clipboard_impl() -> Option<String> {
    match Clipboard::new() {
        Ok(mut cb) => match cb.get_text() {
            Ok(text) => Some(text),
            Err(e) => {
                log::info!("[paster] clipboard empty or error: {}", e);
                None
            }
        },
        Err(e) => {
            log::error!("[paster] cannot open clipboard: {}", e);
            None
        }
    }
}

fn write_clipboard_impl(text: &str) {
    log::info!("[paster] writing to clipboard, length={}", text.len());
    match Clipboard::new() {
        Ok(mut cb) => {
            if let Err(e) = cb.set_text(text) {
                log::error!("[paster] clipboard write failed: {}", e);
            }
        }
        Err(e) => {
            log::error!("[paster] cannot open clipboard: {}", e);
        }
    }
}
