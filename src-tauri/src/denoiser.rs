use crate::error::AppError;
use log::{error, info, warn};
use ndarray::{Array3, Array4};
use ort::session::Session;
use ort::value::Value;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// DTLN constants derived from the ONNX model shapes.
const FRAME_LEN: usize = 512;
const FRAME_SHIFT: usize = 128;
const FFT_SIZE: usize = 512;
const MAG_BINS: usize = FFT_SIZE / 2 + 1; // 257
const HIDDEN_UNITS: usize = 128;

/// Number of warm-up frames to prepend so the LSTM hidden states
/// adapt to the noise profile before processing the actual audio.
/// Without this the first ~1 s gets little/no denoising because
/// the LSTM outputs near-identity masks while its state is all-zeros.
const WARM_UP_FRAMES: usize = 150;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenoiserConfig {
    pub enabled: bool,
    pub model_dir: Option<String>,
}

/// Trait for audio denoising implementations.
pub trait Denoiser {
    fn process(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<f32>, AppError>;
}

/// Passthrough implementation — returns audio unchanged.
pub struct PassthroughDenoiser;

impl Denoiser for PassthroughDenoiser {
    fn process(&self, samples: &[f32], _sample_rate: u32) -> Result<Vec<f32>, AppError> {
        info!("[denoiser] passthrough processing {} samples", samples.len());
        Ok(samples.to_vec())
    }
}

/// DTLN-based denoiser using two ONNX model stages.
///
/// Stage 1: FFT magnitude → LSTM mask → apply mask to complex spectrum → iFFT
/// Stage 2: Time-domain frame → LSTM refinement → output frame
pub struct DtlnDenoiser {
    session1: Mutex<Session>,
    session2: Mutex<Session>,
}

impl DtlnDenoiser {
    /// Load both DTLN ONNX models from `model_dir`.
    /// Expects `dtln_1.onnx` and `dtln_2.onnx` in the directory.
    pub fn new(model_dir: &str) -> Result<Self, AppError> {
        let dir = Path::new(model_dir);
        let path1 = dir.join("dtln_1.onnx");
        let path2 = dir.join("dtln_2.onnx");

        if !path1.exists() {
            error!("[denoiser] dtln_1.onnx not found at {:?}", path1);
            return Err(AppError::NotFound(format!(
                "dtln_1.onnx not found in {}",
                model_dir
            )));
        }
        if !path2.exists() {
            error!("[denoiser] dtln_2.onnx not found at {:?}", path2);
            return Err(AppError::NotFound(format!(
                "dtln_2.onnx not found in {}",
                model_dir
            )));
        }

        info!("[denoiser] loading dtln_1.onnx from {:?}", path1);
        let session1 = Session::builder()
            .map_err(|e| AppError::Io(format!("ort session builder: {}", e)))?
            .commit_from_file(&path1)
            .map_err(|e| AppError::Io(format!("load dtln_1.onnx: {}", e)))?;

        info!("[denoiser] loading dtln_2.onnx from {:?}", path2);
        let session2 = Session::builder()
            .map_err(|e| AppError::Io(format!("ort session builder: {}", e)))?
            .commit_from_file(&path2)
            .map_err(|e| AppError::Io(format!("load dtln_2.onnx: {}", e)))?;

        info!("[denoiser] both DTLN models loaded");
        Ok(Self {
            session1: Mutex::new(session1),
            session2: Mutex::new(session2),
        })
    }
}

impl Denoiser for DtlnDenoiser {
    fn process(&self, samples: &[f32], _sample_rate: u32) -> Result<Vec<f32>, AppError> {
        let input_len = samples.len();
        if input_len == 0 {
            return Ok(vec![]);
        }

        info!("[denoiser] DTLN processing {} samples", input_len);

        // Prepend warm-up segment so the LSTM hidden states are
        // pre-conditioned before processing the real audio start.
        // We mirror-repeat the beginning of the audio as warm-up material.
        // For short recordings (< 2s), scale down warm-up to avoid over-suppression.
        let max_warm_up = WARM_UP_FRAMES * FRAME_SHIFT;
        let warm_up_samples = if input_len < 32000 {
            // Scale: 0 at 0 samples, ramp up to max at 32000 (2s)
            (max_warm_up * input_len) / 32000
        } else {
            max_warm_up
        };
        let warm_up_src_len = input_len.min(warm_up_samples.max(1));
        let mut warm_up = Vec::with_capacity(warm_up_samples);
        while warm_up.len() < warm_up_samples {
            let remaining = warm_up_samples - warm_up.len();
            let chunk_len = remaining.min(warm_up_src_len);
            warm_up.extend_from_slice(&samples[..chunk_len]);
        }
        let mut full_input = Vec::with_capacity(warm_up_samples + input_len);
        full_input.extend_from_slice(&warm_up);
        full_input.extend_from_slice(samples);
        let total_len = full_input.len();

        info!(
            "[denoiser] warm-up: {} samples prepended ({} frames)",
            warm_up_samples, warm_up_samples / FRAME_SHIFT.max(1)
        );

        // Pad so we have at least one full frame and exact overlap-add alignment
        let padded_len = if total_len < FRAME_LEN {
            FRAME_LEN
        } else {
            let extra = (total_len - FRAME_LEN) % FRAME_SHIFT;
            if extra == 0 {
                total_len
            } else {
                total_len + (FRAME_SHIFT - extra)
            }
        };

        let mut padded = vec![0.0f32; padded_len];
        padded[..total_len].copy_from_slice(&full_input);

        let num_frames = (padded_len - FRAME_LEN) / FRAME_SHIFT + 1;

        // Prepare FFT
        let mut planner = FftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(FFT_SIZE);
        let fft_inverse = planner.plan_fft_inverse(FFT_SIZE);

        // Analysis window (sqrt-Hann for perfect reconstruction with overlap-add)
        let window = build_sqrt_hann_window(FRAME_LEN);

        // LSTM hidden states: shape [1, 2, 128, 2]
        let mut h1_data = vec![0.0f32; 2 * HIDDEN_UNITS * 2];
        let mut h2_data = vec![0.0f32; 2 * HIDDEN_UNITS * 2];

        // Output buffer for overlap-add
        let mut output = vec![0.0f32; padded_len];

        let mut session1_guard = self.session1.lock()
            .map_err(|e| AppError::Io(format!("denoiser session1 lock: {}", e)))?;
        let mut session2_guard = self.session2.lock()
            .map_err(|e| AppError::Io(format!("denoiser session2 lock: {}", e)))?;

        for i in 0..num_frames {
            let start = i * FRAME_SHIFT;
            let frame_slice = &padded[start..start + FRAME_LEN];

            // Apply analysis window
            let mut windowed_frame = vec![0.0f32; FRAME_LEN];
            for j in 0..FRAME_LEN {
                windowed_frame[j] = frame_slice[j] * window[j];
            }

            // ---- Stage 1: FFT → magnitude mask → iFFT ----

            // FFT
            let mut fft_buf: Vec<Complex<f32>> = windowed_frame
                .iter()
                .map(|&s| Complex::new(s, 0.0))
                .collect();
            fft_forward.process(&mut fft_buf);

            // Extract magnitude for first MAG_BINS
            let mut mag = vec![0.0f32; MAG_BINS];
            for j in 0..MAG_BINS {
                mag[j] = fft_buf[j].norm();
            }

            // Run stage 1: input magnitude + hidden state → mask + new hidden state
            let mag_array = Array3::from_shape_vec((1, 1, MAG_BINS), mag)
                .map_err(|e| AppError::Io(format!("ndarray shape: {}", e)))?;
            let h1_array =
                Array4::from_shape_vec((1, 2, HIDDEN_UNITS, 2), h1_data.clone())
                    .map_err(|e| AppError::Io(format!("ndarray shape: {}", e)))?;

            let mag_val = Value::from_array(mag_array)
                .map_err(|e| AppError::Io(format!("ort value: {}", e)))?;
            let h1_val = Value::from_array(h1_array)
                .map_err(|e| AppError::Io(format!("ort value: {}", e)))?;

            let out1 = session1_guard
                .run(ort::inputs![
                    "input_2" => mag_val,
                    "input_3" => h1_val,
                ])
                .map_err(|e| AppError::Io(format!("stage1 inference: {}", e)))?;

            // Extract mask and updated hidden state
            let (_mask_shape, mask_data) = out1["activation_2"]
                .try_extract_tensor::<f32>()
                .map_err(|e| AppError::Io(format!("extract mask: {}", e)))?;
            let (_h1_shape, h1_new) = out1["tf_op_layer_stack_2"]
                .try_extract_tensor::<f32>()
                .map_err(|e| AppError::Io(format!("extract h1: {}", e)))?;

            h1_data = h1_new.to_vec();

            // Apply mask to complex spectrum
            for j in 0..MAG_BINS {
                fft_buf[j] *= mask_data[j];
            }
            // Mirror for negative frequencies
            for j in 1..MAG_BINS - 1 {
                fft_buf[FFT_SIZE - j] = fft_buf[j].conj();
            }

            // iFFT
            fft_inverse.process(&mut fft_buf);
            let inv_scale = 1.0 / FFT_SIZE as f32;

            // Stage 1 output frame (time domain after iFFT)
            let mut stage1_frame = vec![0.0f32; FRAME_LEN];
            for j in 0..FRAME_LEN {
                stage1_frame[j] = fft_buf[j].re * inv_scale;
            }

            // ---- Stage 2: time-domain refinement ----

            let frame_array =
                Array3::from_shape_vec((1, 1, FRAME_LEN), stage1_frame)
                    .map_err(|e| AppError::Io(format!("ndarray shape: {}", e)))?;
            let h2_array =
                Array4::from_shape_vec((1, 2, HIDDEN_UNITS, 2), h2_data.clone())
                    .map_err(|e| AppError::Io(format!("ndarray shape: {}", e)))?;

            let frame_val = Value::from_array(frame_array)
                .map_err(|e| AppError::Io(format!("ort value: {}", e)))?;
            let h2_val = Value::from_array(h2_array)
                .map_err(|e| AppError::Io(format!("ort value: {}", e)))?;

            let out2 = session2_guard
                .run(ort::inputs![
                    "input_4" => frame_val,
                    "input_5" => h2_val,
                ])
                .map_err(|e| AppError::Io(format!("stage2 inference: {}", e)))?;

            let (_out_shape, out_frame_data) = out2["conv1d_3"]
                .try_extract_tensor::<f32>()
                .map_err(|e| AppError::Io(format!("extract output: {}", e)))?;
            let (_h2_shape, h2_new) = out2["tf_op_layer_stack_5"]
                .try_extract_tensor::<f32>()
                .map_err(|e| AppError::Io(format!("extract h2: {}", e)))?;

            h2_data = h2_new.to_vec();

            // Overlap-add with synthesis window
            for j in 0..FRAME_LEN {
                output[start + j] += out_frame_data[j] * window[j];
            }
        }

        // Strip warm-up prefix and truncate to original length
        let result: Vec<f32> = output[warm_up_samples..].iter().copied().take(input_len).collect();

        info!("[denoiser] DTLN processing complete, {} samples", result.len());
        Ok(result)
    }
}

/// Build a sqrt-Hann window for DTLN overlap-add reconstruction.
fn build_sqrt_hann_window(len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| {
            let hann = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / len as f32).cos());
            hann.sqrt()
        })
        .collect()
}

/// Process audio through denoiser if enabled, otherwise passthrough.
pub fn process_or_passthrough(
    config: &DenoiserConfig,
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<f32>, AppError> {
    info!(
        "[denoiser] process_or_passthrough enabled={} samples={}",
        config.enabled,
        samples.len()
    );

    if !config.enabled {
        info!("[denoiser] denoiser disabled, passthrough");
        return Ok(samples.to_vec());
    }

    match &config.model_dir {
        Some(dir) => match DtlnDenoiser::new(dir) {
            Ok(d) => d.process(samples, sample_rate),
            Err(e) => {
                warn!(
                    "[denoiser] model not available ({}), falling back to passthrough",
                    e
                );
                Ok(samples.to_vec())
            }
        },
        None => {
            warn!("[denoiser] no model dir configured, passthrough");
            Ok(samples.to_vec())
        }
    }
}
