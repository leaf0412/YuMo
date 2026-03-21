use crate::error::AppResult;
use crate::platform::traits::PlatformAudioCtrl;
use log::{info, warn};

// ---------------------------------------------------------------------------
// WindowsAudioCtrl — PlatformAudioCtrl implementation
// ---------------------------------------------------------------------------
//
// TODO(windows): implement via IAudioEndpointVolume COM interface
// Windows mute detection and control require the COM-based
// IAudioEndpointVolume interface, which is complex to set up without a
// dedicated crate.  For now we return safe defaults and log a warning so
// callers can detect the gap without crashing.

pub struct WindowsAudioCtrl;

impl PlatformAudioCtrl for WindowsAudioCtrl {
    fn is_muted() -> AppResult<bool> {
        info!("[audio_ctrl] is_muted — Windows COM not yet implemented, returning false");
        Ok(false)
    }

    fn set_mute(mute: bool) -> AppResult<()> {
        warn!(
            "[audio_ctrl] set_mute({}) — Windows COM not yet implemented, no-op",
            mute
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn is_system_muted() -> AppResult<bool> {
    WindowsAudioCtrl::is_muted()
}

pub fn set_system_muted(mute: bool) -> AppResult<()> {
    WindowsAudioCtrl::set_mute(mute)
}
