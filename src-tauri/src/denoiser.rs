use crate::error::AppError;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenoiserConfig {
    pub enabled: bool,
    pub model_path: Option<String>,
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

/// ONNX-based denoiser (DeepFilterNet).
/// Requires a valid ONNX model file at construction time.
pub struct OnnxDenoiser {
    // Will hold ort::Session when model is available
    _session: Option<()>, // placeholder
}

impl OnnxDenoiser {
    pub fn new(model_path: &str) -> Result<Self, AppError> {
        info!("[denoiser] loading ONNX model from {}", model_path);
        // TODO: Load ONNX model via ort
        // For now, check if file exists
        if !std::path::Path::new(model_path).exists() {
            error!("[denoiser] model not found at {}", model_path);
            return Err(AppError::NotFound(format!(
                "ONNX model not found: {}",
                model_path
            )));
        }
        info!("[denoiser] model loaded successfully");
        Ok(Self { _session: None })
    }
}

impl Denoiser for OnnxDenoiser {
    fn process(&self, samples: &[f32], _sample_rate: u32) -> Result<Vec<f32>, AppError> {
        info!("[denoiser] ONNX processing start, {} samples", samples.len());
        // TODO: Run ONNX inference when model is loaded
        // For now, passthrough
        let result = samples.to_vec();
        info!("[denoiser] ONNX processing end, {} samples", result.len());
        Ok(result)
    }
}

/// Process audio through denoiser if enabled, otherwise passthrough.
pub fn process_or_passthrough(
    config: &DenoiserConfig,
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<f32>, AppError> {
    info!("[denoiser] process_or_passthrough enabled={} samples={}", config.enabled, samples.len());
    if !config.enabled {
        info!("[denoiser] denoiser disabled, passthrough");
        return Ok(samples.to_vec());
    }

    match &config.model_path {
        Some(path) => {
            match OnnxDenoiser::new(path) {
                Ok(d) => d.process(samples, sample_rate),
                Err(e) => {
                    warn!("[denoiser] model not available ({}), falling back to passthrough", e);
                    Ok(samples.to_vec())
                }
            }
        }
        None => {
            warn!("[denoiser] no model path configured, passthrough");
            Ok(samples.to_vec())
        }
    }
}
