use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformRecorder;
use crate::platform::types::*;
use log::{error, info, warn};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Windows-specific types
// ---------------------------------------------------------------------------

/// Opaque handle returned by `start_recording`.
pub struct RecordingHandle {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
}

unsafe impl Send for RecordingHandle {}

// ---------------------------------------------------------------------------
// WindowsRecorder — PlatformRecorder implementation
// ---------------------------------------------------------------------------

pub struct WindowsRecorder;

impl PlatformRecorder for WindowsRecorder {
    type Handle = RecordingHandle;

    fn list_devices() -> AppResult<Vec<AudioInputDevice>> {
        list_input_devices_impl()
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
    WindowsRecorder::list_devices()
}

pub fn start_recording(device_id: u32) -> AppResult<(RecordingHandle, Receiver<AudioLevel>)> {
    WindowsRecorder::start(device_id)
}

pub fn stop_recording(handle: RecordingHandle) -> AppResult<AudioData> {
    WindowsRecorder::stop(handle)
}

pub fn cancel_recording(handle: RecordingHandle) -> AppResult<()> {
    WindowsRecorder::cancel(handle)
}

// ---------------------------------------------------------------------------
// List input devices (implementation)
// ---------------------------------------------------------------------------

fn list_input_devices_impl() -> AppResult<Vec<AudioInputDevice>> {
    info!("[recorder] list_input_devices");
    let host = cpal::default_host();

    let default_device_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let devices = host
        .input_devices()
        .map_err(|e| {
            error!("[recorder] failed to enumerate input devices: {}", e);
            AppError::Recording(format!("Failed to enumerate input devices: {}", e))
        })?;

    let mut result: Vec<AudioInputDevice> = Vec::new();
    for (idx, device) in devices.enumerate() {
        let name = device.name().unwrap_or_else(|_| format!("Device {}", idx));
        let is_default = name == default_device_name;
        result.push(AudioInputDevice {
            id: idx as u32,
            name,
            is_default,
        });
    }

    info!(
        "[recorder] list_input_devices found {} devices",
        result.len()
    );
    for dev in &result {
        info!(
            "[recorder]   device id={} name={:?} is_default={}",
            dev.id, dev.name, dev.is_default
        );
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Recording via WASAPI / cpal (implementation)
// ---------------------------------------------------------------------------

fn start_recording_impl(
    device_id: u32,
) -> AppResult<(RecordingHandle, Receiver<AudioLevel>)> {
    info!("[recorder] start_recording device_id={}", device_id);

    let host = cpal::default_host();
    let devices: Vec<_> = host
        .input_devices()
        .map_err(|e| {
            error!("[recorder] failed to enumerate devices: {}", e);
            AppError::Recording(format!("Failed to enumerate devices: {}", e))
        })?
        .collect();

    let device = devices.into_iter().nth(device_id as usize).ok_or_else(|| {
        error!("[recorder] device_id={} not found", device_id);
        AppError::Recording(format!("Input device {} not found", device_id))
    })?;

    let device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
    info!("[recorder] using device: {}", device_name);

    // Request 16 kHz mono f32 — the preferred format for Whisper
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let (level_tx, level_rx): (Sender<AudioLevel>, Receiver<AudioLevel>) = mpsc::channel();
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buffer_cb = buffer.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                // Compute audio level
                let mut sum_sq: f32 = 0.0;
                let mut peak: f32 = 0.0;
                for &s in data {
                    sum_sq += s * s;
                    let abs = s.abs();
                    if abs > peak {
                        peak = abs;
                    }
                }
                let rms = if data.is_empty() {
                    0.0
                } else {
                    (sum_sq / data.len() as f32).sqrt()
                };

                // Send level (ignore errors — receiver may be dropped)
                let _ = level_tx.send(AudioLevel { rms, peak });

                // Append samples to buffer
                if let Ok(mut buf) = buffer_cb.lock() {
                    buf.extend_from_slice(data);
                }
            },
            move |err| {
                error!("[recorder] stream error: {}", err);
            },
            None,
        )
        .map_err(|e| {
            error!("[recorder] failed to build input stream: {}", e);
            AppError::Recording(format!("Failed to build input stream: {}", e))
        })?;

    stream.play().map_err(|e| {
        error!("[recorder] failed to start stream: {}", e);
        AppError::Recording(format!("Failed to start stream: {}", e))
    })?;

    info!("[recorder] recording started on device={}", device_name);
    Ok((RecordingHandle { stream, buffer }, level_rx))
}

fn stop_recording_impl(handle: RecordingHandle) -> AppResult<AudioData> {
    info!("[recorder] stop_recording");
    // Dropping the stream stops it
    drop(handle.stream);

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

fn cancel_recording_impl(handle: RecordingHandle) -> AppResult<()> {
    warn!("[recorder] cancel_recording (discarding data)");
    // Drop stream and discard buffer
    drop(handle.stream);
    drop(handle.buffer);
    Ok(())
}
