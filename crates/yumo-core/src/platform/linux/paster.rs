use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformPaster;

// ---------------------------------------------------------------------------
// LinuxPaster — PlatformPaster implementation
//
// Clipboard: arboard (X11 / Wayland)
// Paste simulation:
//   1. xdotool key ctrl+v   (X11, most reliable)
//   2. wtype -M ctrl -k v   (Wayland native)
//   3. enigo Ctrl+V fallback
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
/// Returns Ok(()) on success, Err with a user-facing hint on failure.
pub fn paste_text(text: &str, restore_delay_ms: u64) -> AppResult<()> {
    log::info!("[paster] paste_text restore_delay_ms={}", restore_delay_ms);
    let saved = save_clipboard();
    write_clipboard(text);

    // Small delay to ensure clipboard content is available for paste
    std::thread::sleep(std::time::Duration::from_millis(50));

    let paste_result = simulate_paste_impl();
    if let Err(ref e) = paste_result {
        log::error!("[paster] simulate_paste failed: {}", e);
    }

    if restore_delay_ms > 0 {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(restore_delay_ms));
            restore_clipboard(saved);
        });
    }

    paste_result
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

/// Simulate Ctrl+V paste using the best available method.
///
/// Priority: xdotool (X11) → wtype (Wayland) → enigo (fallback)
fn simulate_paste_impl() -> AppResult<()> {
    // Try xdotool first (works on X11, most common)
    if try_xdotool_paste() {
        return Ok(());
    }

    // Try wtype (native Wayland key simulation)
    if try_wtype_paste() {
        return Ok(());
    }

    // Fallback to enigo
    log::info!("[paster] falling back to enigo for Ctrl+V");
    match simulate_paste_enigo() {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("[paster] all paste methods failed, last error: {}", e);
            Err(AppError::Io(
                "Auto-paste failed: xdotool, wtype, and enigo are all unavailable. \
                 Please install xdotool (X11) or wtype (Wayland): sudo apt install xdotool wtype"
                    .into(),
            ))
        }
    }
}

/// Simulate Ctrl+V via xdotool (X11).
fn try_xdotool_paste() -> bool {
    log::info!("[paster] trying xdotool key ctrl+v");
    match Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                log::info!("[paster] xdotool paste succeeded");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("[paster] xdotool failed: {}", stderr.trim());
                false
            }
        }
        Err(e) => {
            log::info!("[paster] xdotool not available: {}", e);
            false
        }
    }
}

/// Simulate Ctrl+V via wtype (Wayland).
fn try_wtype_paste() -> bool {
    log::info!("[paster] trying wtype -M ctrl -k v");
    match Command::new("wtype")
        .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                log::info!("[paster] wtype paste succeeded");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("[paster] wtype failed: {}", stderr.trim());
                false
            }
        }
        Err(e) => {
            log::info!("[paster] wtype not available: {}", e);
            false
        }
    }
}

/// Simulate Ctrl+V via enigo (XTest-based, X11 only).
fn simulate_paste_enigo() -> AppResult<()> {
    log::info!("[paster] simulating Ctrl+V via enigo");
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
