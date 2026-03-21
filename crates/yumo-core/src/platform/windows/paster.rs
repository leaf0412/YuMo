use arboard::Clipboard;
use crate::error::{AppError, AppResult};
use crate::mask;
use crate::platform::traits::PlatformPaster;
use log::{error, info};

// ---------------------------------------------------------------------------
// WindowsPaster — PlatformPaster implementation
// ---------------------------------------------------------------------------

pub struct WindowsPaster;

impl PlatformPaster for WindowsPaster {
    fn read_clipboard() -> AppResult<Option<String>> {
        read_clipboard_impl()
    }

    fn write_clipboard(text: &str) -> AppResult<()> {
        write_clipboard_impl(text)
    }

    fn save_clipboard() -> AppResult<Option<String>> {
        save_clipboard_impl()
    }

    fn restore_clipboard(saved: Option<String>) -> AppResult<()> {
        restore_clipboard_impl(saved)
    }

    fn simulate_paste() -> AppResult<()> {
        simulate_paste_impl()
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn read_clipboard() -> Option<String> {
    read_clipboard_impl().unwrap_or(None)
}

pub fn write_clipboard(text: &str) {
    if let Err(e) = write_clipboard_impl(text) {
        error!("[paster] write_clipboard failed: {}", e);
    }
}

pub fn save_clipboard() -> Option<String> {
    save_clipboard_impl().unwrap_or(None)
}

pub fn restore_clipboard(saved: Option<String>) {
    if let Err(e) = restore_clipboard_impl(saved) {
        error!("[paster] restore_clipboard failed: {}", e);
    }
}

pub fn simulate_paste() {
    if let Err(e) = simulate_paste_impl() {
        error!("[paster] simulate_paste failed: {}", e);
    }
}

/// Full paste flow: save clipboard, write text, simulate Ctrl+V, then
/// asynchronously restore the original clipboard after `restore_delay_ms`.
pub fn paste_text(text: &str, restore_delay_ms: u64) {
    info!(
        "[paster] paste_text start, text={} restore_delay_ms={}",
        mask::mask_text(text),
        restore_delay_ms
    );
    let saved = save_clipboard();
    write_clipboard(text);
    simulate_paste();

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

fn read_clipboard_impl() -> AppResult<Option<String>> {
    info!("[paster] reading clipboard");
    let mut clipboard = Clipboard::new().map_err(|e| {
        error!("[paster] failed to open clipboard: {}", e);
        AppError::Io(format!("Failed to open clipboard: {}", e))
    })?;
    match clipboard.get_text() {
        Ok(text) => {
            info!("[paster] clipboard read ok, length={}", text.len());
            Ok(Some(text))
        }
        Err(_) => {
            info!("[paster] clipboard is empty or not text");
            Ok(None)
        }
    }
}

fn write_clipboard_impl(text: &str) -> AppResult<()> {
    info!(
        "[paster] writing to clipboard, text={}",
        mask::mask_text(text)
    );
    let mut clipboard = Clipboard::new().map_err(|e| {
        error!("[paster] failed to open clipboard: {}", e);
        AppError::Io(format!("Failed to open clipboard: {}", e))
    })?;
    clipboard.set_text(text).map_err(|e| {
        error!("[paster] failed to write clipboard: {}", e);
        AppError::Io(format!("Failed to write clipboard: {}", e))
    })
}

fn save_clipboard_impl() -> AppResult<Option<String>> {
    info!("[paster] saving clipboard snapshot");
    read_clipboard_impl()
}

fn restore_clipboard_impl(saved: Option<String>) -> AppResult<()> {
    info!("[paster] restoring clipboard, has_saved={}", saved.is_some());
    if let Some(text) = saved {
        write_clipboard_impl(&text)?;
    }
    Ok(())
}

fn simulate_paste_impl() -> AppResult<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    info!("[paster] simulating Ctrl+V paste");
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        error!("[paster] failed to create Enigo: {}", e);
        AppError::Io(format!("Failed to create Enigo: {}", e))
    })?;

    enigo.key(Key::Control, Direction::Press).map_err(|e| {
        error!("[paster] key press Control failed: {}", e);
        AppError::Io(format!("Key press Control failed: {}", e))
    })?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| {
            error!("[paster] key click v failed: {}", e);
            AppError::Io(format!("Key click v failed: {}", e))
        })?;
    enigo.key(Key::Control, Direction::Release).map_err(|e| {
        error!("[paster] key release Control failed: {}", e);
        AppError::Io(format!("Key release Control failed: {}", e))
    })?;

    Ok(())
}
