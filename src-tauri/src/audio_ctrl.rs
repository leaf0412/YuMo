use coreaudio_sys::*;
use log::{error, info};
use std::mem;

pub fn default_output_device_id() -> AudioDeviceID {
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
            device_id
        } else {
            error!("[audio_ctrl] failed to get default output device, status={}", status);
            kAudioObjectUnknown
        }
    }
}

pub fn is_system_muted() -> bool {
    info!("[audio_ctrl] checking system mute state");
    let device_id = default_output_device_id();
    if device_id == kAudioObjectUnknown {
        return false;
    }

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
        result
    }
}

pub fn set_system_muted(mute: bool) -> bool {
    info!("[audio_ctrl] set_system_muted mute={}", mute);
    let device_id = default_output_device_id();
    if device_id == kAudioObjectUnknown {
        error!("[audio_ctrl] cannot set mute, unknown device");
        return false;
    }

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
        let ok = status == 0;
        if ok {
            info!("[audio_ctrl] mute set successfully");
        } else {
            error!("[audio_ctrl] failed to set mute, status={}", status);
        }
        ok
    }
}
