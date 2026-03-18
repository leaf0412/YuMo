use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelProvider {
    Local,
    MlxFunASR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    pub languages: Vec<String>,
    pub download_url: String,
    pub is_downloaded: bool,
    pub provider: ModelProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-tiny".into(),
            name: "Tiny (Multilingual)".into(),
            size_mb: 75,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-base.en".into(),
            name: "Base (English)".into(),
            size_mb: 142,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-base".into(),
            name: "Base (Multilingual)".into(),
            size_mb: 142,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-small.en".into(),
            name: "Small (English)".into(),
            size_mb: 466,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-small".into(),
            name: "Small (Multilingual)".into(),
            size_mb: 466,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-medium.en".into(),
            name: "Medium (English)".into(),
            size_mb: 1457,
            languages: vec!["en".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-medium".into(),
            name: "Medium (Multilingual)".into(),
            size_mb: 1457,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
        ModelInfo {
            id: "ggml-large-v3".into(),
            name: "Large v3 (Multilingual)".into(),
            size_mb: 2952,
            languages: vec!["multi".into()],
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin".into(),
            is_downloaded: false,
            provider: ModelProvider::Local,
            model_repo: None,
            description: None,
        },
    ]
}

pub fn predefined_mlx_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "mlx-funasr-nano-8bit".into(),
            name: "MLX Fun-ASR Nano (8-bit)".into(),
            size_mb: 2000,
            languages: vec!["multi".into()],
            download_url: String::new(),
            is_downloaded: false,
            provider: ModelProvider::MlxFunASR,
            model_repo: Some("mlx-community/Fun-ASR-MLT-Nano-2512-8bit".into()),
            description: Some("8-bit quantized, fast inference".into()),
        },
        ModelInfo {
            id: "mlx-funasr-nano-bf16".into(),
            name: "MLX Fun-ASR Nano (BF16)".into(),
            size_mb: 4000,
            languages: vec!["multi".into()],
            download_url: String::new(),
            is_downloaded: false,
            provider: ModelProvider::MlxFunASR,
            model_repo: Some("mlx-community/Fun-ASR-MLT-Nano-2512-bf16".into()),
            description: Some("BF16 precision, higher quality".into()),
        },
        ModelInfo {
            id: "mlx-qwen3-asr-0.6b-bf16".into(),
            name: "Qwen3-ASR 0.6B (BF16)".into(),
            size_mb: 1200,
            languages: vec!["multi".into()],
            download_url: String::new(),
            is_downloaded: false,
            provider: ModelProvider::MlxFunASR,
            model_repo: Some("mlx-community/Qwen3-ASR-0.6B-bf16".into()),
            description: Some("Qwen3, 30+ languages".into()),
        },
        ModelInfo {
            id: "mlx-qwen3-asr-0.6b-8bit".into(),
            name: "Qwen3-ASR 0.6B (8-bit)".into(),
            size_mb: 700,
            languages: vec!["multi".into()],
            download_url: String::new(),
            is_downloaded: false,
            provider: ModelProvider::MlxFunASR,
            model_repo: Some("mlx-community/Qwen3-ASR-0.6B-8bit".into()),
            description: Some("Qwen3 quantized, fast".into()),
        },
    ]
}

/// Check if an MLX model is available in the HuggingFace hub cache.
/// Looks for any `.safetensors` file under the model's snapshots directory.
pub fn check_mlx_model_downloaded(model_repo: &str) -> bool {
    let cache_name = model_repo.replace('/', "--");
    let cache_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".cache/huggingface/hub")
        .join(format!("models--{}", cache_name))
        .join("snapshots");

    if !cache_path.exists() {
        return false;
    }

    if let Ok(entries) = std::fs::read_dir(&cache_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Ok(files) = std::fs::read_dir(entry.path()) {
                    for file in files.flatten() {
                        if file
                            .path()
                            .extension()
                            .map(|e| e == "safetensors")
                            .unwrap_or(false)
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

pub fn all_models(models_dir: &Path) -> Vec<ModelInfo> {
    let local_models = check_downloaded_models(models_dir);

    let mut mlx_models = predefined_mlx_models();
    for model in &mut mlx_models {
        if let Some(repo) = &model.model_repo {
            model.is_downloaded = check_mlx_model_downloaded(repo);
        }
    }

    local_models.into_iter().chain(mlx_models).collect()
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

/// Transcribe audio via the MLX FunASR daemon.
/// Writes samples to a temp WAV file and sends the path to the daemon.
pub fn transcribe_via_daemon(
    daemon: &crate::daemon::DaemonManager,
    samples: &[f32],
    sample_rate: u32,
    language: &str,
) -> Result<TranscriptionResult, AppError> {
    let start = std::time::Instant::now();

    // Write samples to a temp WAV file (daemon expects a file path)
    let tmp_dir = std::env::temp_dir().join("voiceink");
    std::fs::create_dir_all(&tmp_dir)?;
    let wav_path = tmp_dir.join("recording.wav");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&wav_path, spec)
        .map_err(|e| AppError::Io(e.to_string()))?;
    for &s in samples {
        writer.write_sample(s).map_err(|e| AppError::Io(e.to_string()))?;
    }
    writer.finalize().map_err(|e| AppError::Io(e.to_string()))?;

    let cmd = serde_json::json!({
        "action": "transcribe",
        "audio": wav_path.to_string_lossy(),
        "language": language,
        "max_tokens": 1900,
        "temperature": 0.0,
    });

    let resp = daemon.send_command(&cmd)?;

    // Cleanup temp file
    let _ = std::fs::remove_file(&wav_path);

    if resp.status == "success" {
        Ok(TranscriptionResult {
            text: resp.text.unwrap_or_default(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    } else {
        Err(AppError::Transcription(
            resp.error.unwrap_or_else(|| "Transcription failed".into())
        ))
    }
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
