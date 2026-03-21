use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformAudioCtrl;

// ---------------------------------------------------------------------------
// LinuxAudioCtrl — PlatformAudioCtrl implementation via pactl
//
// Uses PulseAudio / PipeWire's `pactl` CLI for best-effort mute control.
// If pactl is not available the call degrades gracefully rather than
// returning an error that would surface to the user.
// ---------------------------------------------------------------------------

pub struct LinuxAudioCtrl;

impl PlatformAudioCtrl for LinuxAudioCtrl {
    fn is_muted() -> AppResult<bool> {
        log::info!("[audio_ctrl] is_muted: querying @DEFAULT_SINK@ via pactl");
        match Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
        {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout);
                let muted = text.contains("yes");
                log::info!("[audio_ctrl] is_muted={}", muted);
                Ok(muted)
            }
            Err(e) => {
                // pactl may not be installed (pure ALSA systems, etc.)
                log::warn!("[audio_ctrl] pactl unavailable, assuming unmuted: {}", e);
                Ok(false)
            }
        }
    }

    fn set_mute(mute: bool) -> AppResult<()> {
        let val = if mute { "1" } else { "0" };
        log::info!("[audio_ctrl] set_mute mute={} (pactl)", mute);
        Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", val])
            .status()
            .map_err(|e| AppError::Io(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn is_system_muted() -> AppResult<bool> {
    LinuxAudioCtrl::is_muted()
}

pub fn set_system_muted(mute: bool) -> AppResult<()> {
    LinuxAudioCtrl::set_mute(mute)
}
