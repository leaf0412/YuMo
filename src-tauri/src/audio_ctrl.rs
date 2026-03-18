use coreaudio_sys::*;
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
            device_id
        } else {
            kAudioObjectUnknown
        }
    }
}

pub fn is_system_muted() -> bool {
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
        status == 0 && muted != 0
    }
}

pub fn set_system_muted(mute: bool) -> bool {
    let device_id = default_output_device_id();
    if device_id == kAudioObjectUnknown {
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
        status == 0
    }
}
