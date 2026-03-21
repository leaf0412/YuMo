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
    native_sample_rate: u32,
    native_channels: u16,
}

// SAFETY: RecordingHandle is created on one thread and moved (via stop/cancel) to exactly
// one consumer thread. The Stream is never concurrently accessed — it is moved whole.
// Cross-thread buffer access is protected by Arc<Mutex<Vec<f32>>>.
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
                idx, name, is_default
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

        let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
        log::info!("[recorder] selected device: {:?}", device_name);

        // Try 16kHz mono first; fall back to device default if unsupported
        let (config, native_sr, native_ch) = match device.supported_input_configs() {
            Ok(mut configs) => {
                let target_sr = cpal::SampleRate(16000);
                let has_16k_mono = configs.any(|c| {
                    c.channels() == 1
                        && c.min_sample_rate() <= target_sr
                        && c.max_sample_rate() >= target_sr
                });

                if has_16k_mono {
                    log::info!("[recorder] device supports 16kHz mono natively");
                    (cpal::StreamConfig {
                        channels: 1,
                        sample_rate: target_sr,
                        buffer_size: cpal::BufferSize::Default,
                    }, 16000u32, 1u16)
                } else {
                    let default_config = device.default_input_config().map_err(|e| {
                        AppError::Recording(format!("No supported input config: {}", e))
                    })?;
                    let sr = default_config.sample_rate().0;
                    let ch = default_config.channels();
                    log::info!("[recorder] 16kHz mono unsupported, using device default: {}Hz {}ch", sr, ch);
                    (cpal::StreamConfig {
                        channels: ch,
                        sample_rate: cpal::SampleRate(sr),
                        buffer_size: cpal::BufferSize::Default,
                    }, sr, ch)
                }
            }
            Err(_) => {
                log::warn!("[recorder] cannot query supported configs, trying 16kHz mono");
                (cpal::StreamConfig {
                    channels: 1,
                    sample_rate: cpal::SampleRate(16000),
                    buffer_size: cpal::BufferSize::Default,
                }, 16000, 1)
            }
        };

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();
        let (level_tx, level_rx) = mpsc::channel();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.extend_from_slice(data);
                    }
                    if !data.is_empty() {
                        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / data.len() as f32).sqrt();
                        let peak = data.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
                        let _ = level_tx.send(AudioLevel { rms, peak });
                    }
                },
                move |err| {
                    log::error!("[recorder] cpal stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AppError::Recording(e.to_string()))?;

        stream.play().map_err(|e| AppError::Recording(e.to_string()))?;

        log::info!("[recorder] stream started config={}Hz {}ch", native_sr, native_ch);
        Ok((RecordingHandle { stream, buffer, native_sample_rate: native_sr, native_channels: native_ch }, level_rx))
    }

    fn stop(handle: Self::Handle) -> AppResult<AudioData> {
        log::info!("[recorder] stop_recording");
        let native_sr = handle.native_sample_rate;
        let native_ch = handle.native_channels;
        drop(handle.stream);

        let mut samples = handle
            .buffer
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .clone();

        // Convert to mono if multi-channel
        if native_ch > 1 {
            log::info!("[recorder] converting {}ch to mono", native_ch);
            let ch = native_ch as usize;
            samples = samples.chunks(ch).map(|frame| {
                frame.iter().sum::<f32>() / ch as f32
            }).collect();
        }

        // Resample to 16kHz if native rate differs
        if native_sr != 16000 {
            log::info!("[recorder] resampling {}Hz -> 16000Hz ({} samples)", native_sr, samples.len());
            samples = linear_resample(&samples, native_sr, 16000);
        }

        let duration_secs = samples.len() as f64 / 16_000.0;
        log::info!(
            "[recorder] stop_recording samples={} duration={:.2}s",
            samples.len(), duration_secs
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
