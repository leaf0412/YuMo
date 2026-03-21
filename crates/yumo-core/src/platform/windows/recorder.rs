use std::sync::mpsc::Receiver;
use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformRecorder;
use crate::platform::types::*;

/// Windows recording handle (WASAPI) — placeholder
pub struct RecordingHandle;
unsafe impl Send for RecordingHandle {}

pub struct WindowsRecorder;

impl PlatformRecorder for WindowsRecorder {
    type Handle = RecordingHandle;

    fn list_devices() -> AppResult<Vec<AudioInputDevice>> {
        Err(AppError::Recording("Windows audio not yet implemented".into()))
    }

    fn start(_device_id: u32) -> AppResult<(Self::Handle, Receiver<AudioLevel>)> {
        Err(AppError::Recording("Windows audio not yet implemented".into()))
    }

    fn stop(_handle: Self::Handle) -> AppResult<AudioData> {
        Err(AppError::Recording("Windows audio not yet implemented".into()))
    }

    fn cancel(_handle: Self::Handle) -> AppResult<()> {
        Err(AppError::Recording("Windows audio not yet implemented".into()))
    }
}

pub fn list_input_devices() -> AppResult<Vec<AudioInputDevice>> {
    WindowsRecorder::list_devices()
}

pub fn start_recording(_device_id: u32) -> AppResult<(RecordingHandle, Receiver<AudioLevel>)> {
    WindowsRecorder::start(_device_id)
}

pub fn stop_recording(handle: RecordingHandle) -> AppResult<AudioData> {
    WindowsRecorder::stop(handle)
}

pub fn cancel_recording(handle: RecordingHandle) -> AppResult<()> {
    WindowsRecorder::cancel(handle)
}
