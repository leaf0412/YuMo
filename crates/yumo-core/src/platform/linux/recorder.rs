use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformRecorder;
use crate::platform::types::*;

// ---------------------------------------------------------------------------
// Linux recording handle — wraps a live cpal stream and the shared buffer
// ---------------------------------------------------------------------------

pub struct RecordingHandle {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
}

// cpal::Stream contains a raw pointer internally; we only access it from the
// owning thread (start / stop / cancel), so Send is safe here.
unsafe impl Send for RecordingHandle {}

// ---------------------------------------------------------------------------
// LinuxRecorder — PlatformRecorder implementation (ALSA / PulseAudio / PipeWire)
// ---------------------------------------------------------------------------

pub struct LinuxRecorder;

impl PlatformRecorder for LinuxRecorder {
    type Handle = RecordingHandle;

    fn list_devices() -> AppResult<Vec<AudioInputDevice>> {
        log::info!("[recorder] list_input_devices (linux/cpal)");
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|d| d.name().ok());

        let mut devices = Vec::new();
        for (idx, device) in host
            .input_devices()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .enumerate()
        {
            let name = device.name().unwrap_or_else(|_| format!("Device {}", idx));
            let is_default = default_name.as_deref() == Some(name.as_str());
            log::info!(
                "[recorder]   device idx={} name={:?} is_default={}",
                idx,
                name,
                is_default
            );
            devices.push(AudioInputDevice {
                id: idx as u32,
                name,
                is_default,
            });
        }
        log::info!("[recorder] found {} input device(s)", devices.len());
        Ok(devices)
    }

    fn start(device_id: u32) -> AppResult<(Self::Handle, Receiver<AudioLevel>)> {
        log::info!("[recorder] start_recording device_id={}", device_id);
        let host = cpal::default_host();

        let device = host
            .input_devices()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .nth(device_id as usize)
            .ok_or_else(|| {
                AppError::Recording(format!("Input device {} not found", device_id))
            })?;

        log::info!(
            "[recorder] selected device: {:?}",
            device.name().unwrap_or_else(|_| "<unknown>".into())
        );

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16_000),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();
        let (level_tx, level_rx) = mpsc::channel();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Append samples to the shared buffer
                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.extend_from_slice(data);
                    }

                    // Compute and publish audio level
                    if !data.is_empty() {
                        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / data.len() as f32).sqrt();
                        let peak = data
                            .iter()
                            .map(|s| s.abs())
                            .fold(0.0_f32, f32::max);
                        let _ = level_tx.send(AudioLevel { rms, peak });
                    }
                },
                move |err| {
                    log::error!("[recorder] cpal stream error: {}", err);
                },
                None, // no timeout
            )
            .map_err(|e| AppError::Recording(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AppError::Recording(e.to_string()))?;

        log::info!("[recorder] stream started at 16 kHz mono f32");
        Ok((RecordingHandle { stream, buffer }, level_rx))
    }

    fn stop(handle: Self::Handle) -> AppResult<AudioData> {
        log::info!("[recorder] stop_recording");
        // Dropping the stream stops capture
        drop(handle.stream);

        let samples = handle
            .buffer
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .clone();

        let duration_secs = samples.len() as f64 / 16_000.0;
        log::info!(
            "[recorder] stop_recording samples={} duration={:.2}s",
            samples.len(),
            duration_secs
        );
        Ok(AudioData {
            pcm_samples: samples,
            sample_rate: 16_000,
            channels: 1,
        })
    }

    fn cancel(handle: Self::Handle) -> AppResult<()> {
        log::warn!("[recorder] cancel_recording (discarding data)");
        drop(handle.stream);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn list_input_devices() -> AppResult<Vec<AudioInputDevice>> {
    LinuxRecorder::list_devices()
}

pub fn start_recording(
    device_id: u32,
) -> AppResult<(RecordingHandle, Receiver<AudioLevel>)> {
    LinuxRecorder::start(device_id)
}

pub fn stop_recording(handle: RecordingHandle) -> AppResult<AudioData> {
    LinuxRecorder::stop(handle)
}

pub fn cancel_recording(handle: RecordingHandle) -> AppResult<()> {
    LinuxRecorder::cancel(handle)
}
