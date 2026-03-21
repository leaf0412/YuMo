use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformAudioCtrl;

pub struct LinuxAudioCtrl;

impl PlatformAudioCtrl for LinuxAudioCtrl {
    fn is_muted() -> AppResult<bool> {
        Err(AppError::Recording("Linux audio control not yet implemented".into()))
    }

    fn set_mute(_mute: bool) -> AppResult<()> {
        Err(AppError::Recording("Linux audio control not yet implemented".into()))
    }
}

pub fn is_system_muted() -> AppResult<bool> {
    LinuxAudioCtrl::is_muted()
}

pub fn set_system_muted(mute: bool) -> AppResult<()> {
    LinuxAudioCtrl::set_mute(mute)
}
