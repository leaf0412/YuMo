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
    native_sample_rate: u32,
    native_channels: u16,
}

// SAFETY: RecordingHandle is created on one thread and moved (via stop/cancel) to exactly
// one consumer thread. The Stream is never concurrently accessed — it is moved whole.
// Cross-thread buffer access is protected by Arc<Mutex<Vec<f32>>>.
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

    // Try 16 kHz mono first; fall back to device default if unsupported
    let (config, native_sr, native_ch) = match device.supported_input_configs() {
        Ok(mut configs) => {
            // Check if 16kHz mono is directly supported
            let target_sr = cpal::SampleRate(16000);
            let has_16k_mono = configs.any(|c| {
                c.channels() == 1
                    && c.min_sample_rate() <= target_sr
                    && c.max_sample_rate() >= target_sr
            });

            if has_16k_mono {
                info!("[recorder] device supports 16kHz mono natively");
                (cpal::StreamConfig {
                    channels: 1,
                    sample_rate: target_sr,
                    buffer_size: cpal::BufferSize::Default,
                }, 16000u32, 1u16)
            } else {
                // Use device default config; we'll resample later
                let default_config = device.default_input_config().map_err(|e| {
                    AppError::Recording(format!("No supported input config: {}", e))
                })?;
                let sr = default_config.sample_rate().0;
                let ch = default_config.channels();
                info!("[recorder] 16kHz mono unsupported, using device default: {}Hz {}ch", sr, ch);
                (cpal::StreamConfig {
                    channels: ch,
                    sample_rate: cpal::SampleRate(sr),
                    buffer_size: cpal::BufferSize::Default,
                }, sr, ch)
            }
        }
        Err(_) => {
            // Fallback: try 16kHz mono anyway
            warn!("[recorder] cannot query supported configs, trying 16kHz mono");
            (cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(16000),
                buffer_size: cpal::BufferSize::Default,
            }, 16000, 1)
        }
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

    info!("[recorder] recording started on device={} config={}Hz {}ch", device_name, native_sr, native_ch);
    Ok((RecordingHandle { stream, buffer, native_sample_rate: native_sr, native_channels: native_ch }, level_rx))
}

fn stop_recording_impl(handle: RecordingHandle) -> AppResult<AudioData> {
    info!("[recorder] stop_recording");
    let native_sr = handle.native_sample_rate;
    let native_ch = handle.native_channels;
    // Dropping the stream stops it
    drop(handle.stream);

    let mut samples = handle
        .buffer
        .lock()
        .map_err(|e| {
            error!("[recorder] failed to lock buffer: {}", e);
            AppError::Recording(e.to_string())
        })?
        .clone();

    // Convert to mono if multi-channel
    if native_ch > 1 {
        info!("[recorder] converting {}ch to mono", native_ch);
        let ch = native_ch as usize;
        samples = samples.chunks(ch).map(|frame| {
            frame.iter().sum::<f32>() / ch as f32
        }).collect();
    }

    // Resample to 16kHz if native rate differs
    if native_sr != 16000 {
        info!("[recorder] resampling {}Hz -> 16000Hz ({} samples)", native_sr, samples.len());
        samples = linear_resample(&samples, native_sr, 16000);
    }

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
    drop(handle.stream);
    drop(handle.buffer);
    Ok(())
}

/// Simple linear interpolation resampling.
fn linear_resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        let s = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32
        } else {
            samples[idx.min(samples.len() - 1)]
        };
        out.push(s);
    }
    out
}
