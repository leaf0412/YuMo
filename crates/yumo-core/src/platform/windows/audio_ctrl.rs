use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformAudioCtrl;

pub struct WindowsAudioCtrl;

impl PlatformAudioCtrl for WindowsAudioCtrl {
    fn is_muted() -> AppResult<bool> {
        Err(AppError::Recording("Windows audio control not yet implemented".into()))
    }

    fn set_mute(_mute: bool) -> AppResult<()> {
        Err(AppError::Recording("Windows audio control not yet implemented".into()))
    }
}

pub fn is_system_muted() -> AppResult<bool> {
    WindowsAudioCtrl::is_muted()
}

pub fn set_system_muted(mute: bool) -> AppResult<()> {
    WindowsAudioCtrl::set_mute(mute)
}
