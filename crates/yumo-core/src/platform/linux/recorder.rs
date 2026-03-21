use std::sync::mpsc::Receiver;
use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformRecorder;
use crate::platform::types::*;

/// Linux recording handle (ALSA/PipeWire) — placeholder
pub struct RecordingHandle;
unsafe impl Send for RecordingHandle {}

pub struct LinuxRecorder;

impl PlatformRecorder for LinuxRecorder {
    type Handle = RecordingHandle;

    fn list_devices() -> AppResult<Vec<AudioInputDevice>> {
        Err(AppError::Recording("Linux audio not yet implemented".into()))
    }

    fn start(_device_id: u32) -> AppResult<(Self::Handle, Receiver<AudioLevel>)> {
        Err(AppError::Recording("Linux audio not yet implemented".into()))
    }

    fn stop(_handle: Self::Handle) -> AppResult<AudioData> {
        Err(AppError::Recording("Linux audio not yet implemented".into()))
    }

    fn cancel(_handle: Self::Handle) -> AppResult<()> {
        Err(AppError::Recording("Linux audio not yet implemented".into()))
    }
}

pub fn list_input_devices() -> AppResult<Vec<AudioInputDevice>> {
    LinuxRecorder::list_devices()
}

pub fn start_recording(_device_id: u32) -> AppResult<(RecordingHandle, Receiver<AudioLevel>)> {
    LinuxRecorder::start(_device_id)
}

pub fn stop_recording(handle: RecordingHandle) -> AppResult<AudioData> {
    LinuxRecorder::stop(handle)
}

pub fn cancel_recording(handle: RecordingHandle) -> AppResult<()> {
    LinuxRecorder::cancel(handle)
}
