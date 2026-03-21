use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformPaster;

// ---------------------------------------------------------------------------
// LinuxPaster — PlatformPaster implementation
//
// Clipboard: arboard (X11 / Wayland)
// Paste simulation: enigo 0.2 Ctrl+V (X11 / Wayland)
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
        Ok(save_clipboard_impl())
    }

    fn restore_clipboard(saved: Option<String>) -> AppResult<()> {
        restore_clipboard_impl(saved);
        Ok(())
    }

    fn simulate_paste() -> AppResult<()> {
        simulate_paste_impl()
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn read_clipboard() -> Option<String> {
    read_clipboard_impl()
}

pub fn write_clipboard(text: &str) {
    write_clipboard_impl(text);
}

pub fn save_clipboard() -> Option<String> {
    save_clipboard_impl()
}

pub fn restore_clipboard(saved: Option<String>) {
    restore_clipboard_impl(saved);
}

pub fn simulate_paste() {
    let _ = simulate_paste_impl();
}

/// Full paste flow: save clipboard → write text → Ctrl+V → restore after delay.
pub fn paste_text(text: &str, restore_delay_ms: u64) {
    log::info!("[paster] paste_text restore_delay_ms={}", restore_delay_ms);
    let saved = save_clipboard();
    write_clipboard(text);
    let _ = simulate_paste_impl();

    if restore_delay_ms > 0 {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(restore_delay_ms));
            restore_clipboard(saved);
        });
    }
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

fn read_clipboard_impl() -> Option<String> {
    log::info!("[paster] reading clipboard");
    match Clipboard::new() {
        Ok(mut cb) => match cb.get_text() {
            Ok(text) => {
                log::info!("[paster] clipboard read ok, length={}", text.len());
                Some(text)
            }
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

fn save_clipboard_impl() -> Option<String> {
    log::info!("[paster] saving clipboard snapshot");
    read_clipboard_impl()
}

fn restore_clipboard_impl(saved: Option<String>) {
    log::info!("[paster] restoring clipboard, has_saved={}", saved.is_some());
    if let Some(text) = saved {
        write_clipboard_impl(&text);
    }
}

fn simulate_paste_impl() -> AppResult<()> {
    log::info!("[paster] simulating Ctrl+V paste");
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::Io(format!("enigo init failed: {}", e)))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| AppError::Io(format!("enigo key press failed: {}", e)))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| AppError::Io(format!("enigo key click failed: {}", e)))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| AppError::Io(format!("enigo key release failed: {}", e)))?;
    Ok(())
}
