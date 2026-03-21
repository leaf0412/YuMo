use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformAudioCtrl;
use coreaudio_sys::*;
use log::{error, info};
use std::mem;

// ---------------------------------------------------------------------------
// MacosAudioCtrl — PlatformAudioCtrl implementation
// ---------------------------------------------------------------------------

pub struct MacosAudioCtrl;

impl PlatformAudioCtrl for MacosAudioCtrl {
    fn is_muted() -> AppResult<bool> {
        is_system_muted_impl()
    }

    fn set_mute(mute: bool) -> AppResult<()> {
        set_system_muted_impl(mute)
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn is_system_muted() -> AppResult<bool> {
    MacosAudioCtrl::is_muted()
}

pub fn set_system_muted(mute: bool) -> AppResult<()> {
    MacosAudioCtrl::set_mute(mute)
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

fn default_output_device_id() -> AppResult<AudioDeviceID> {
    let mut device_id: AudioDeviceID = kAudioObjectUnknown;
    let mut size = mem::size_of::<AudioDeviceID>() as u32;
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut device_id as *mut _ as *mut _,
        );
        if status == 0 {
            info!("[audio_ctrl] default output device_id={}", device_id);
            Ok(device_id)
        } else {
            error!("[audio_ctrl] failed to get default output device, status={}", status);
            Err(AppError::Io(format!("Failed to get default output device, status={}", status)))
        }
    }
}

fn is_system_muted_impl() -> AppResult<bool> {
    info!("[audio_ctrl] checking system mute state");
    let device_id = default_output_device_id()?;

    let mut muted: u32 = 0;
    let mut size = mem::size_of::<u32>() as u32;
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyMute,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let status = AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut muted as *mut _ as *mut _,
        );
        let result = status == 0 && muted != 0;
        info!("[audio_ctrl] system muted={}", result);
        Ok(result)
    }
}

fn set_system_muted_impl(mute: bool) -> AppResult<()> {
    info!("[audio_ctrl] set_system_muted mute={}", mute);
    let device_id = default_output_device_id()?;

    let muted: u32 = if mute { 1 } else { 0 };
    let size = mem::size_of::<u32>() as u32;
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyMute,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let status = AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            size,
            &muted as *const _ as *const _,
        );
        if status == 0 {
            info!("[audio_ctrl] mute set successfully");
            Ok(())
        } else {
            error!("[audio_ctrl] failed to set mute, status={}", status);
            Err(AppError::Io(format!("Failed to set mute, status={}", status)))
        }
    }
}
