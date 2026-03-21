use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformRecorder;
use crate::platform::types::*;
use coreaudio_sys::*;
use log::{error, info, warn};
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// macOS-specific types
// ---------------------------------------------------------------------------

/// Opaque handle returned by `start_recording`.
pub struct RecordingHandle {
    audio_unit: AudioUnit,
    buffer: Arc<Mutex<Vec<f32>>>,
    /// Prevent the callback data from being freed until we stop.
    _callback_box: *mut InputCallbackData,
}

unsafe impl Send for RecordingHandle {}

struct InputCallbackData {
    audio_unit: AudioUnit,
    buffer: Arc<Mutex<Vec<f32>>>,
    level_tx: Sender<AudioLevel>,
}

// ---------------------------------------------------------------------------
// MacosRecorder — PlatformRecorder implementation
// ---------------------------------------------------------------------------

pub struct MacosRecorder;

impl PlatformRecorder for MacosRecorder {
    type Handle = RecordingHandle;

    fn list_devices() -> AppResult<Vec<AudioInputDevice>> {
        Ok(list_input_devices_impl())
    }

    fn start(device_id: u32) -> AppResult<(Self::Handle, Receiver<AudioLevel>)> {
        start_recording_impl(device_id)
    }

    fn stop(handle: Self::Handle) -> AppResult<AudioData> {
        stop_recording_impl(handle)
    }

    fn cancel(handle: Self::Handle) -> AppResult<()> {
        cancel_recording_impl(handle)
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn list_input_devices() -> AppResult<Vec<AudioInputDevice>> {
    MacosRecorder::list_devices()
}

pub fn start_recording(
    device_id: u32,
) -> Result<(RecordingHandle, Receiver<AudioLevel>), AppError> {
    MacosRecorder::start(device_id)
}

pub fn stop_recording(handle: RecordingHandle) -> Result<AudioData, AppError> {
    MacosRecorder::stop(handle)
}

pub fn cancel_recording(handle: RecordingHandle) -> Result<(), AppError> {
    MacosRecorder::cancel(handle)
}

// ---------------------------------------------------------------------------
// List input devices (implementation)
// ---------------------------------------------------------------------------

fn list_input_devices_impl() -> Vec<AudioInputDevice> {
    info!("[recorder] list_input_devices");
    let default_id = default_input_device_id();
    let device_ids = all_device_ids();
    let mut result = Vec::new();

    for &did in &device_ids {
        if !has_input_streams(did) {
            continue;
        }
        if let Some(name) = device_name(did) {
            result.push(AudioInputDevice {
                id: did,
                name,
                is_default: did == default_id,
            });
        }
    }
    info!(
        "[recorder] list_input_devices found {} devices, default_id={}",
        result.len(),
        default_id
    );
    for dev in &result {
        info!(
            "[recorder]   device id={} name={:?} is_default={}",
            dev.id, dev.name, dev.is_default
        );
    }
    result
}

fn default_input_device_id() -> AudioDeviceID {
    let mut device_id: AudioDeviceID = kAudioObjectUnknown;
    let mut size = mem::size_of::<AudioDeviceID>() as u32;
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultInputDevice,
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

fn all_device_ids() -> Vec<AudioDeviceID> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size: u32 = 0;
    unsafe {
        let status = AudioObjectGetPropertyDataSize(
            kAudioObjectSystemObject,
            &address,
            0,
            std::ptr::null(),
            &mut size,
        );
        if status != 0 || size == 0 {
            return Vec::new();
        }
        let count = size as usize / mem::size_of::<AudioDeviceID>();
        let mut ids = vec![0u32; count];
        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            ids.as_mut_ptr() as *mut _,
        );
        if status != 0 {
            return Vec::new();
        }
        ids
    }
}

fn has_input_streams(device_id: AudioDeviceID) -> bool {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyStreamConfiguration,
        mScope: kAudioDevicePropertyScopeInput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size: u32 = 0;
    unsafe {
        let status = AudioObjectGetPropertyDataSize(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
        );
        if status != 0 || size == 0 {
            return false;
        }
        let mut buf = vec![0u8; size as usize];
        let status = AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            buf.as_mut_ptr() as *mut _,
        );
        if status != 0 {
            return false;
        }
        let buffer_list = buf.as_ptr() as *const AudioBufferList;
        let n_buffers = (*buffer_list).mNumberBuffers;
        if n_buffers == 0 {
            return false;
        }
        let buffers = std::slice::from_raw_parts(
            &(*buffer_list).mBuffers as *const AudioBuffer,
            n_buffers as usize,
        );
        buffers.iter().any(|b| b.mNumberChannels > 0)
    }
}

fn device_name(device_id: AudioDeviceID) -> Option<String> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioObjectPropertyName,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut name_ref: CFStringRef = std::ptr::null();
    let mut size = mem::size_of::<CFStringRef>() as u32;
    unsafe {
        let status = AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut name_ref as *mut _ as *mut _,
        );
        if status != 0 || name_ref.is_null() {
            return None;
        }
        let cf_name = cfstring_to_string(name_ref);
        CFRelease(name_ref as *const _);
        cf_name
    }
}

unsafe fn cfstring_to_string(cf: CFStringRef) -> Option<String> {
    let len = CFStringGetLength(cf);
    if len == 0 {
        return Some(String::new());
    }
    let range = CFRange {
        location: 0,
        length: len,
    };
    let mut buf_len: CFIndex = 0;
    CFStringGetBytes(
        cf,
        range,
        kCFStringEncodingUTF8,
        0,
        false as Boolean,
        std::ptr::null_mut(),
        0,
        &mut buf_len,
    );
    if buf_len <= 0 {
        return None;
    }
    let mut buf = vec![0u8; buf_len as usize];
    CFStringGetBytes(
        cf,
        range,
        kCFStringEncodingUTF8,
        0,
        false as Boolean,
        buf.as_mut_ptr(),
        buf_len,
        std::ptr::null_mut(),
    );
    String::from_utf8(buf).ok()
}

// ---------------------------------------------------------------------------
// Recording via AUHAL (implementation)
// ---------------------------------------------------------------------------

/// Start recording from the given device at 16 kHz mono f32.
///
/// Returns a handle to stop/cancel the recording and a receiver for
/// real-time audio level updates.
fn start_recording_impl(
    device_id: u32,
) -> Result<(RecordingHandle, Receiver<AudioLevel>), AppError> {
    info!("[recorder] start_recording device_id={}", device_id);
    let (level_tx, level_rx) = mpsc::channel();
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    unsafe {
        // 1. Create AUHAL AudioUnit
        let mut comp_desc = AudioComponentDescription {
            componentType: kAudioUnitType_Output,
            componentSubType: kAudioUnitSubType_HALOutput,
            componentManufacturer: kAudioUnitManufacturer_Apple,
            componentFlags: 0,
            componentFlagsMask: 0,
        };
        let component = AudioComponentFindNext(std::ptr::null_mut(), &mut comp_desc);
        if component.is_null() {
            error!("[recorder] cannot find HALOutput AudioComponent");
            return Err(AppError::Recording(
                "Cannot find HALOutput AudioComponent".into(),
            ));
        }

        let mut audio_unit: AudioUnit = std::ptr::null_mut();
        let status = AudioComponentInstanceNew(component, &mut audio_unit);
        if status != 0 {
            error!("[recorder] AudioComponentInstanceNew failed: {}", status);
            return Err(AppError::Recording(format!(
                "AudioComponentInstanceNew failed: {status}"
            )));
        }

        // 2. Enable input on bus 1, disable output on bus 0
        let enable: u32 = 1;
        let disable: u32 = 0;
        AudioUnitSetProperty(
            audio_unit,
            kAudioOutputUnitProperty_EnableIO,
            kAudioUnitScope_Input,
            1,
            &enable as *const _ as *const _,
            mem::size_of::<u32>() as u32,
        );
        AudioUnitSetProperty(
            audio_unit,
            kAudioOutputUnitProperty_EnableIO,
            kAudioUnitScope_Output,
            0,
            &disable as *const _ as *const _,
            mem::size_of::<u32>() as u32,
        );

        // 3. Set the input device
        let dev_id: AudioDeviceID = device_id;
        let status = AudioUnitSetProperty(
            audio_unit,
            kAudioOutputUnitProperty_CurrentDevice,
            kAudioUnitScope_Global,
            0,
            &dev_id as *const _ as *const _,
            mem::size_of::<AudioDeviceID>() as u32,
        );
        if status != 0 {
            error!(
                "[recorder] cannot set input device {}: {}",
                device_id, status
            );
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "Cannot set input device: {status}"
            )));
        }

        // 4. Set desired output format on input bus (output scope, bus 1)
        let desired_format = AudioStreamBasicDescription {
            mSampleRate: 16000.0,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 4,
            mFramesPerPacket: 1,
            mBytesPerFrame: 4,
            mChannelsPerFrame: 1,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        info!("[recorder] setting stream format: sample_rate=16000 channels=1 bits=32 (f32)");
        let status = AudioUnitSetProperty(
            audio_unit,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Output,
            1,
            &desired_format as *const _ as *const _,
            mem::size_of::<AudioStreamBasicDescription>() as u32,
        );
        if status != 0 {
            error!("[recorder] cannot set stream format: {}", status);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "Cannot set stream format: {status}"
            )));
        }

        // 5. Allocate callback data with the AudioUnit pointer
        let cb_data = Box::new(InputCallbackData {
            audio_unit,
            buffer: buffer.clone(),
            level_tx,
        });
        let cb_ptr = Box::into_raw(cb_data);

        // 6. Set input callback
        let callback_struct = AURenderCallbackStruct {
            inputProc: Some(input_callback),
            inputProcRefCon: cb_ptr as *mut _,
        };
        let status = AudioUnitSetProperty(
            audio_unit,
            kAudioOutputUnitProperty_SetInputCallback,
            kAudioUnitScope_Global,
            0,
            &callback_struct as *const _ as *const _,
            mem::size_of::<AURenderCallbackStruct>() as u32,
        );
        if status != 0 {
            error!("[recorder] cannot set input callback: {}", status);
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "Cannot set input callback: {status}"
            )));
        }

        // 7. Initialize and start
        let status = AudioUnitInitialize(audio_unit);
        if status != 0 {
            error!("[recorder] AudioUnitInitialize failed: {}", status);
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "AudioUnitInitialize failed: {status}"
            )));
        }

        let status = AudioOutputUnitStart(audio_unit);
        if status != 0 {
            error!("[recorder] AudioOutputUnitStart failed: {}", status);
            AudioUnitUninitialize(audio_unit);
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "AudioOutputUnitStart failed: {status}"
            )));
        }

        info!("[recorder] recording started on device_id={}", device_id);
        Ok((
            RecordingHandle {
                audio_unit,
                buffer,
                _callback_box: cb_ptr,
            },
            level_rx,
        ))
    }
}

/// Stop recording and return the captured audio data.
fn stop_recording_impl(handle: RecordingHandle) -> Result<AudioData, AppError> {
    info!("[recorder] stop_recording");
    unsafe {
        AudioOutputUnitStop(handle.audio_unit);
        AudioUnitUninitialize(handle.audio_unit);
        AudioComponentInstanceDispose(handle.audio_unit);
        let _ = Box::from_raw(handle._callback_box);
    }
    let samples = handle
        .buffer
        .lock()
        .map_err(|e| {
            error!("[recorder] failed to lock buffer: {}", e);
            AppError::Recording(e.to_string())
        })?
        .clone();
    let duration_secs = samples.len() as f64 / 16000.0;
    info!(
        "[recorder] stop_recording samples={} duration={:.2}s",
        samples.len(),
        duration_secs
    );
    Ok(AudioData {
        pcm_samples: samples,
        sample_rate: 16000,
        channels: 1,
    })
}

/// Cancel recording, discarding all data.
fn cancel_recording_impl(handle: RecordingHandle) -> Result<(), AppError> {
    warn!("[recorder] cancel_recording (discarding data)");
    let _ = stop_recording_impl(handle)?;
    Ok(())
}

/// CoreAudio input render callback — called on the audio I/O thread.
unsafe extern "C" fn input_callback(
    in_ref_con: *mut std::os::raw::c_void,
    io_action_flags: *mut AudioUnitRenderActionFlags,
    in_time_stamp: *const AudioTimeStamp,
    in_bus_number: u32,
    in_number_frames: u32,
    _io_data: *mut AudioBufferList,
) -> OSStatus {
    let cb_data = &*(in_ref_con as *const InputCallbackData);

    // Prepare a buffer for AudioUnitRender to fill
    let byte_size = in_number_frames * 4; // f32 = 4 bytes
    let mut data_buf = vec![0f32; in_number_frames as usize];
    let mut buf_list = AudioBufferList {
        mNumberBuffers: 1,
        mBuffers: [AudioBuffer {
            mNumberChannels: 1,
            mDataByteSize: byte_size,
            mData: data_buf.as_mut_ptr() as *mut _,
        }],
    };

    let status = AudioUnitRender(
        cb_data.audio_unit,
        io_action_flags,
        in_time_stamp,
        in_bus_number,
        in_number_frames,
        &mut buf_list,
    );

    if status != 0 {
        return status;
    }

    // Compute audio level
    let mut sum_sq: f32 = 0.0;
    let mut peak: f32 = 0.0;
    for &s in &data_buf {
        sum_sq += s * s;
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
    }
    let rms = (sum_sq / in_number_frames as f32).sqrt();

    // Send level (ignore errors — receiver might be dropped)
    let _ = cb_data.level_tx.send(AudioLevel { rms, peak });

    // Append samples to buffer
    if let Ok(mut buf) = cb_data.buffer.lock() {
        buf.extend_from_slice(&data_buf);
    }

    0
}
