use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    pub languages: Vec<String>,
    pub download_url: String,
    pub is_downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
}

pub fn predefined_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "ggml-tiny.en".into(),
            name: "Tiny (English)".into(),
            size_mb: 75,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-tiny".into(),
            name: "Tiny (Multilingual)".into(),
            size_mb: 75,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-base.en".into(),
            name: "Base (English)".into(),
            size_mb: 142,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-base".into(),
            name: "Base (Multilingual)".into(),
            size_mb: 142,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-small.en".into(),
            name: "Small (English)".into(),
            size_mb: 466,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-small".into(),
            name: "Small (Multilingual)".into(),
            size_mb: 466,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-medium.en".into(),
            name: "Medium (English)".into(),
            size_mb: 1457,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-medium".into(),
            name: "Medium (Multilingual)".into(),
            size_mb: 1457,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin".into(),
            is_downloaded: false,
        },
        ModelInfo {
            id: "ggml-large-v3".into(),
            name: "Large v3 (Multilingual)".into(),
            size_mb: 2952,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin".into(),
            is_downloaded: false,
        },
    ]
}

/// Return the path for a model, preferring a file that already exists in any
/// known directory.  Falls back to the app-local models dir.
pub fn model_path(models_dir: &Path, model_id: &str) -> PathBuf {
    let bin_name = format!("{}.bin", model_id);
    let dirs = model_search_dirs(models_dir);
    for d in &dirs {
        let p = d.join(&bin_name);
        if p.exists() {
            return p;
        }
    }
    // Default: app-local dir (used for new downloads)
    models_dir.join(bin_name)
}

/// Directories where VoiceInk (native) or this app may store downloaded models.
fn model_search_dirs(app_models_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![app_models_dir.to_path_buf()];
    if let Some(home) = dirs::home_dir() {
        // VoiceInk native app model location
        dirs.push(
            home.join("Library/Application Support/com.prakashjoshipax.VoiceInk/Models"),
        );
        // whisper.cpp build tree sometimes used by VoiceInk
        dirs.push(home.join("VoiceInk-Dependencies/whisper.cpp/models"));
    }
    dirs
}

/// Check whether a model file exists in any of the known directories.
fn find_model_file(dirs: &[PathBuf], model_id: &str) -> bool {
    let bin_name = format!("{}.bin", model_id);
    dirs.iter().any(|d| d.join(&bin_name).exists())
}

pub fn check_downloaded_models(models_dir: &Path) -> Vec<ModelInfo> {
    let dirs = model_search_dirs(models_dir);
    let mut models = predefined_models();
    for model in &mut models {
        model.is_downloaded = find_model_file(&dirs, &model.id);
    }
    models
}

pub fn format_text(text: &str, auto_capitalize: bool, _auto_punctuate: bool) -> String {
    let trimmed = text.trim().to_string();
    if auto_capitalize {
        crate::text_processor::capitalize_sentences(&trimmed)
    } else {
        trimmed
    }
}

/// Load a whisper model from disk.
pub fn load_model(path: &Path) -> Result<whisper_rs::WhisperContext, AppError> {
    let ctx = whisper_rs::WhisperContext::new_with_params(
        path.to_str()
            .ok_or(AppError::InvalidInput("Invalid model path".into()))?,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|e| AppError::Transcription(format!("Failed to load model: {}", e)))?;
    Ok(ctx)
}

/// Transcribe audio samples using a loaded whisper model.
pub fn transcribe(
    ctx: &whisper_rs::WhisperContext,
    samples: &[f32],
    _sample_rate: u32,
    language: &str,
) -> Result<TranscriptionResult, AppError> {
    let mut params =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    let start = std::time::Instant::now();

    let mut state = ctx
        .create_state()
        .map_err(|e| AppError::Transcription(format!("Failed to create state: {}", e)))?;
    state
        .full(params, samples)
        .map_err(|e| AppError::Transcription(format!("Transcription failed: {}", e)))?;

    let num_segments = state.full_n_segments();

    let mut text = String::new();
    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i) {
            let seg_text = segment
                .to_str()
                .map_err(|e| AppError::Transcription(format!("Failed to get segment {}: {}", i, e)))?;
            text.push_str(seg_text);
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
        duration_ms,
    })
}
