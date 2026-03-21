use std::sync::mpsc::Receiver;
use crate::error::AppResult;
use super::types::*;

pub trait PlatformRecorder {
    type Handle: Send;
    fn list_devices() -> AppResult<Vec<AudioInputDevice>>;
    fn start(device_id: u32) -> AppResult<(Self::Handle, Receiver<AudioLevel>)>;
    fn stop(handle: Self::Handle) -> AppResult<AudioData>;
    fn cancel(handle: Self::Handle) -> AppResult<()>;
}

pub trait PlatformAudioCtrl {
    fn is_muted() -> AppResult<bool>;
    fn set_mute(mute: bool) -> AppResult<()>;
}

pub trait PlatformPaster {
    fn read_clipboard() -> AppResult<Option<String>>;
    fn write_clipboard(text: &str) -> AppResult<()>;
    fn save_clipboard() -> AppResult<Option<String>>;
    fn restore_clipboard(saved: Option<String>) -> AppResult<()>;
    fn simulate_paste() -> AppResult<()>;
}

pub trait PlatformPermissions {
    fn check_microphone() -> bool;
    fn check_accessibility() -> bool;
    fn check_all() -> PermissionStatus;
    fn request_microphone() -> AppResult<()>;
    fn open_microphone_settings() -> AppResult<()>;
    fn open_accessibility_settings() -> AppResult<()>;
}

pub trait PlatformKeychain {
    fn store_key(service: &str, account: &str, password: &str) -> AppResult<()>;
    fn get_key(service: &str, account: &str) -> AppResult<Option<String>>;
    fn delete_key(service: &str, account: &str) -> AppResult<()>;
}
