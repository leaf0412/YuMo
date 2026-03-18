use crate::error::AppError;
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
        // TODO: Load ONNX model via ort
        // For now, check if file exists
        if !std::path::Path::new(model_path).exists() {
            return Err(AppError::NotFound(format!(
                "ONNX model not found: {}",
                model_path
            )));
        }
        Ok(Self { _session: None })
    }
}

impl Denoiser for OnnxDenoiser {
    fn process(&self, samples: &[f32], _sample_rate: u32) -> Result<Vec<f32>, AppError> {
        // TODO: Run ONNX inference when model is loaded
        // For now, passthrough
        Ok(samples.to_vec())
    }
}

/// Process audio through denoiser if enabled, otherwise passthrough.
pub fn process_or_passthrough(
    config: &DenoiserConfig,
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<f32>, AppError> {
    if !config.enabled {
        return Ok(samples.to_vec());
    }

    match &config.model_path {
        Some(path) => {
            match OnnxDenoiser::new(path) {
                Ok(d) => d.process(samples, sample_rate),
                Err(_) => {
                    // Model not available, passthrough
                    Ok(samples.to_vec())
                }
            }
        }
        None => Ok(samples.to_vec()),
    }
}
