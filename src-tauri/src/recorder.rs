use crate::error::AppError;
use coreaudio_sys::*;
use serde::{Deserialize, Serialize};
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInputDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct AudioData {
    pub pcm_samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
}

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
// List input devices
// ---------------------------------------------------------------------------

pub fn list_input_devices() -> Vec<AudioInputDevice> {
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
// Recording via AUHAL
// ---------------------------------------------------------------------------

/// Start recording from the given device at 16 kHz mono f32.
///
/// Returns a handle to stop/cancel the recording and a receiver for
/// real-time audio level updates.
pub fn start_recording(
    device_id: u32,
) -> Result<(RecordingHandle, Receiver<AudioLevel>), AppError> {
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
            return Err(AppError::Recording(
                "Cannot find HALOutput AudioComponent".into(),
            ));
        }

        let mut audio_unit: AudioUnit = std::ptr::null_mut();
        let status = AudioComponentInstanceNew(component, &mut audio_unit);
        if status != 0 {
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
        let status = AudioUnitSetProperty(
            audio_unit,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Output,
            1,
            &desired_format as *const _ as *const _,
            mem::size_of::<AudioStreamBasicDescription>() as u32,
        );
        if status != 0 {
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
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "Cannot set input callback: {status}"
            )));
        }

        // 7. Initialize and start
        let status = AudioUnitInitialize(audio_unit);
        if status != 0 {
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "AudioUnitInitialize failed: {status}"
            )));
        }

        let status = AudioOutputUnitStart(audio_unit);
        if status != 0 {
            AudioUnitUninitialize(audio_unit);
            let _ = Box::from_raw(cb_ptr);
            AudioComponentInstanceDispose(audio_unit);
            return Err(AppError::Recording(format!(
                "AudioOutputUnitStart failed: {status}"
            )));
        }

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
pub fn stop_recording(handle: RecordingHandle) -> Result<AudioData, AppError> {
    unsafe {
        AudioOutputUnitStop(handle.audio_unit);
        AudioUnitUninitialize(handle.audio_unit);
        AudioComponentInstanceDispose(handle.audio_unit);
        let _ = Box::from_raw(handle._callback_box);
    }
    let samples = handle
        .buffer
        .lock()
        .map_err(|e| AppError::Recording(e.to_string()))?
        .clone();
    Ok(AudioData {
        pcm_samples: samples,
        sample_rate: 16000,
        channels: 1,
    })
}

/// Cancel recording, discarding all data.
pub fn cancel_recording(handle: RecordingHandle) -> Result<(), AppError> {
    let _ = stop_recording(handle)?;
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

// ---------------------------------------------------------------------------
// WAV writing
// ---------------------------------------------------------------------------

pub fn save_wav(data: &AudioData, path: &std::path::Path) -> Result<(), AppError> {
    let spec = hound::WavSpec {
        channels: data.channels,
        sample_rate: data.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| AppError::Io(e.to_string()))?;
    for &sample in &data.pcm_samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(s)
            .map_err(|e| AppError::Io(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| AppError::Io(e.to_string()))?;
    Ok(())
}
