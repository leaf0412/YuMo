pub mod keymap;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub code: String,
    pub key: String,
    pub is_modifier: bool,
}

pub type HotkeyCallback = Arc<dyn Fn() + Send + Sync>;

const NO_KEYCODE: u16 = u16::MAX;

pub struct HotkeyListener {
    target_keycode: Arc<AtomicU16>,
    target_is_modifier: Arc<AtomicBool>,
    on_hotkey: Arc<Mutex<Option<HotkeyCallback>>>,
    on_escape: Arc<Mutex<Option<HotkeyCallback>>>,
    running: AtomicBool,
}

impl HotkeyListener {
    pub fn new() -> Self {
        Self {
            target_keycode: Arc::new(AtomicU16::new(NO_KEYCODE)),
            target_is_modifier: Arc::new(AtomicBool::new(false)),
            on_hotkey: Arc::new(Mutex::new(None)),
            on_escape: Arc::new(Mutex::new(None)),
            running: AtomicBool::new(false),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.swap(true, Ordering::SeqCst) {
            info!("[hotkey] listener already running");
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        macos::start_event_tap(
            self.target_keycode.clone(),
            self.target_is_modifier.clone(),
            self.on_hotkey.clone(),
            self.on_escape.clone(),
        )?;

        info!("[hotkey] native listener started");
        Ok(())
    }

    pub fn register(
        &self,
        config: &HotkeyConfig,
        callback: HotkeyCallback,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let native_keycode = keymap::browser_code_to_macos(&config.code)
            .ok_or_else(|| format!("unsupported key code: {}", config.code))?;

        info!(
            "[hotkey] registering: code={} native=0x{:02X} is_modifier={}",
            config.code, native_keycode, config.is_modifier
        );

        *self.on_hotkey.lock().map_err(|e| e.to_string())? = Some(callback);
        self.target_is_modifier
            .store(config.is_modifier, Ordering::SeqCst);
        self.target_keycode
            .store(native_keycode, Ordering::SeqCst);
        Ok(())
    }

    pub fn unregister(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("[hotkey] unregistering hotkey");
        self.target_keycode.store(NO_KEYCODE, Ordering::SeqCst);
        *self.on_hotkey.lock().map_err(|e| e.to_string())? = None;
        Ok(())
    }

    pub fn register_escape(
        &self,
        callback: HotkeyCallback,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("[hotkey] registering Escape callback");
        *self.on_escape.lock().map_err(|e| e.to_string())? = Some(callback);
        Ok(())
    }

    pub fn unregister_escape(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("[hotkey] unregistering Escape callback");
        *self.on_escape.lock().map_err(|e| e.to_string())? = None;
        Ok(())
    }
}
