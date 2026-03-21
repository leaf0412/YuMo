use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformPaster;

pub struct LinuxPaster;

impl PlatformPaster for LinuxPaster {
    fn read_clipboard() -> AppResult<Option<String>> {
        Err(AppError::Recording("Linux clipboard not yet implemented".into()))
    }

    fn write_clipboard(_text: &str) -> AppResult<()> {
        Err(AppError::Recording("Linux clipboard not yet implemented".into()))
    }

    fn save_clipboard() -> AppResult<Option<String>> {
        Err(AppError::Recording("Linux clipboard not yet implemented".into()))
    }

    fn restore_clipboard(_saved: Option<String>) -> AppResult<()> {
        Err(AppError::Recording("Linux clipboard not yet implemented".into()))
    }

    fn simulate_paste() -> AppResult<()> {
        Err(AppError::Recording("Linux paste simulation not yet implemented".into()))
    }
}

pub fn read_clipboard() -> Option<String> {
    None
}

pub fn write_clipboard(_text: &str) {}

pub fn save_clipboard() -> Option<String> {
    None
}

pub fn restore_clipboard(_saved: Option<String>) {}

pub fn simulate_paste() {}

pub fn paste_text(_text: &str, _restore_delay_ms: u64) {}
