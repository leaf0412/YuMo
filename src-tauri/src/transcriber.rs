use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Model Provider — all supported transcription backends
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelProvider {
    Local,
    MlxWhisper,
    MlxFunASR,
    Groq,
    Deepgram,
    ElevenLabs,
    Mistral,
    Gemini,
    Soniox,
}

impl ModelProvider {
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local | Self::MlxWhisper | Self::MlxFunASR)
    }

    pub fn is_cloud(&self) -> bool {
        matches!(
            self,
            Self::Groq | Self::Deepgram | Self::ElevenLabs | Self::Mistral | Self::Gemini | Self::Soniox
        )
    }

    pub fn needs_daemon(&self) -> bool {
        matches!(self, Self::MlxWhisper | Self::MlxFunASR)
    }
}

// ---------------------------------------------------------------------------
// Model Filter — for UI filtering
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelFilter {
    Recommended,
    Local,
    Cloud,
}

// ---------------------------------------------------------------------------
// Model Info — unified model descriptor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    /// Language code → display name, e.g. {"en": "English", "zh": "中文"}
    pub supported_languages: HashMap<String, String>,
    /// Kept for backward compat with frontend; derived from supported_languages keys
    pub languages: Vec<String>,
    pub download_url: String,
    pub is_downloaded: bool,
    pub provider: ModelProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Speed rating 1-10 (10 = fastest)
    pub speed: u8,
    /// Accuracy rating 1-10 (10 = best)
    pub accuracy: u8,
    /// Whether this model shows in the "Recommended" filter
    pub is_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Language maps
// ---------------------------------------------------------------------------

fn english_only() -> HashMap<String, String> {
    HashMap::from([("en".into(), "English".into())])
}

fn multilingual() -> HashMap<String, String> {
    HashMap::from([
        ("en".into(), "English".into()),
        ("zh".into(), "中文".into()),
        ("ja".into(), "日本語".into()),
        ("ko".into(), "한국어".into()),
        ("fr".into(), "Français".into()),
        ("de".into(), "Deutsch".into()),
        ("es".into(), "Español".into()),
        ("ru".into(), "Русский".into()),
        ("pt".into(), "Português".into()),
        ("it".into(), "Italiano".into()),
    ])
}

fn langs_to_vec(map: &HashMap<String, String>) -> Vec<String> {
    if map.len() > 1 { vec!["multi".into()] } else { map.keys().cloned().collect() }
}

// ---------------------------------------------------------------------------
// Helper to build a ModelInfo
// ---------------------------------------------------------------------------

struct M {
    id: &'static str,
    name: &'static str,
    size_mb: u32,
    langs: HashMap<String, String>,
    url: &'static str,
    provider: ModelProvider,
    repo: Option<&'static str>,
    desc: Option<&'static str>,
    speed: u8,
    accuracy: u8,
    recommended: bool,
}

impl M {
    fn build(self) -> ModelInfo {
        let languages = langs_to_vec(&self.langs);
        ModelInfo {
            id: self.id.into(),
            name: self.name.into(),
            size_mb: self.size_mb,
            supported_languages: self.langs,
            languages,
            download_url: self.url.into(),
            is_downloaded: false,
            provider: self.provider,
            model_repo: self.repo.map(Into::into),
            description: self.desc.map(Into::into),
            speed: self.speed,
            accuracy: self.accuracy,
            is_recommended: self.recommended,
        }
    }
}

// ---------------------------------------------------------------------------
// All predefined models (single source of truth)
// ---------------------------------------------------------------------------

pub fn all_predefined_models() -> Vec<ModelInfo> {
    vec![
        // ---- Local Whisper (ggml) ----
        M { id: "ggml-tiny.en", name: "Tiny (English)", size_mb: 75, langs: english_only(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Fastest, lowest accuracy"),
            speed: 10, accuracy: 3, recommended: false }.build(),
        M { id: "ggml-tiny", name: "Tiny (Multilingual)", size_mb: 75, langs: multilingual(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Fastest multilingual"),
            speed: 10, accuracy: 3, recommended: false }.build(),
        M { id: "ggml-base.en", name: "Base (English)", size_mb: 142, langs: english_only(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Good balance for English"),
            speed: 8, accuracy: 5, recommended: true }.build(),
        M { id: "ggml-base", name: "Base (Multilingual)", size_mb: 142, langs: multilingual(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Good balance, multilingual"),
            speed: 8, accuracy: 5, recommended: false }.build(),
        M { id: "ggml-small.en", name: "Small (English)", size_mb: 466, langs: english_only(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Higher accuracy English"),
            speed: 6, accuracy: 7, recommended: false }.build(),
        M { id: "ggml-small", name: "Small (Multilingual)", size_mb: 466, langs: multilingual(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Higher accuracy, multilingual"),
            speed: 6, accuracy: 7, recommended: false }.build(),
        M { id: "ggml-medium.en", name: "Medium (English)", size_mb: 1457, langs: english_only(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("High accuracy English"),
            speed: 4, accuracy: 8, recommended: false }.build(),
        M { id: "ggml-medium", name: "Medium (Multilingual)", size_mb: 1457, langs: multilingual(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("High accuracy, multilingual"),
            speed: 4, accuracy: 8, recommended: false }.build(),
        M { id: "ggml-large-v3", name: "Large v3 (Multilingual)", size_mb: 2952, langs: multilingual(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
            provider: ModelProvider::Local, repo: None, desc: Some("Best accuracy, slowest"),
            speed: 2, accuracy: 10, recommended: true }.build(),

        // ---- MLX Whisper (GPU accelerated) ----
        M { id: "mlx-whisper-large-v3", name: "MLX Whisper Large v3", size_mb: 3000, langs: multilingual(),
            url: "", provider: ModelProvider::MlxWhisper,
            repo: Some("mlx-community/whisper-large-v3-mlx"),
            desc: Some("GPU accelerated, best quality"), speed: 5, accuracy: 10, recommended: true }.build(),
        M { id: "mlx-whisper-distil-large-v3", name: "MLX Whisper Distil Large v3", size_mb: 1500, langs: multilingual(),
            url: "", provider: ModelProvider::MlxWhisper,
            repo: Some("mlx-community/distil-whisper-large-v3"),
            desc: Some("GPU accelerated, fast + accurate"), speed: 7, accuracy: 9, recommended: true }.build(),
        M { id: "mlx-whisper-small", name: "MLX Whisper Small", size_mb: 500, langs: multilingual(),
            url: "", provider: ModelProvider::MlxWhisper,
            repo: Some("mlx-community/whisper-small-mlx"),
            desc: Some("GPU accelerated, lightweight"), speed: 8, accuracy: 6, recommended: false }.build(),

        // ---- MLX FunASR ----
        M { id: "mlx-funasr-nano-8bit", name: "MLX Fun-ASR Nano (8-bit)", size_mb: 2000, langs: multilingual(),
            url: "", provider: ModelProvider::MlxFunASR,
            repo: Some("mlx-community/Fun-ASR-MLT-Nano-2512-8bit"),
            desc: Some("8-bit quantized, fast inference"), speed: 9, accuracy: 7, recommended: true }.build(),
        M { id: "mlx-funasr-nano-bf16", name: "MLX Fun-ASR Nano (BF16)", size_mb: 4000, langs: multilingual(),
            url: "", provider: ModelProvider::MlxFunASR,
            repo: Some("mlx-community/Fun-ASR-MLT-Nano-2512-bf16"),
            desc: Some("BF16 precision, higher quality"), speed: 7, accuracy: 8, recommended: false }.build(),
        M { id: "mlx-qwen3-asr-0.6b-bf16", name: "Qwen3-ASR 0.6B (BF16)", size_mb: 1200, langs: multilingual(),
            url: "", provider: ModelProvider::MlxFunASR,
            repo: Some("mlx-community/Qwen3-ASR-0.6B-bf16"),
            desc: Some("Qwen3 architecture, 30+ languages"), speed: 8, accuracy: 8, recommended: false }.build(),
        M { id: "mlx-qwen3-asr-0.6b-8bit", name: "Qwen3-ASR 0.6B (8-bit)", size_mb: 700, langs: multilingual(),
            url: "", provider: ModelProvider::MlxFunASR,
            repo: Some("mlx-community/Qwen3-ASR-0.6B-8bit"),
            desc: Some("Qwen3 quantized, fast"), speed: 9, accuracy: 7, recommended: false }.build(),

        // ---- Cloud models ----
        M { id: "groq-whisper-large-v3", name: "Groq Whisper Large v3", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::Groq, repo: None,
            desc: Some("Ultra-fast cloud transcription via Groq"), speed: 10, accuracy: 9, recommended: true }.build(),
        M { id: "deepgram-nova-2", name: "Deepgram Nova-2", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::Deepgram, repo: None,
            desc: Some("Enterprise-grade speech-to-text"), speed: 9, accuracy: 9, recommended: false }.build(),
        M { id: "elevenlabs-scribe", name: "ElevenLabs Scribe", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::ElevenLabs, repo: None,
            desc: Some("High quality transcription"), speed: 8, accuracy: 9, recommended: false }.build(),
        M { id: "mistral-asr", name: "Mistral ASR", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::Mistral, repo: None,
            desc: Some("Mistral speech recognition"), speed: 8, accuracy: 8, recommended: false }.build(),
        M { id: "gemini-asr", name: "Gemini ASR", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::Gemini, repo: None,
            desc: Some("Google Gemini speech recognition"), speed: 8, accuracy: 9, recommended: false }.build(),
        M { id: "soniox-asr", name: "Soniox ASR", size_mb: 0, langs: multilingual(),
            url: "", provider: ModelProvider::Soniox, repo: None,
            desc: Some("Real-time speech recognition"), speed: 9, accuracy: 8, recommended: false }.build(),
    ]
}

// ---------------------------------------------------------------------------
// Backward-compatible accessors
// ---------------------------------------------------------------------------

/// Local whisper models only (backward compat).
pub fn predefined_models() -> Vec<ModelInfo> {
    all_predefined_models()
        .into_iter()
        .filter(|m| matches!(m.provider, ModelProvider::Local))
        .collect()
}

/// MLX FunASR models only (backward compat).
pub fn predefined_mlx_models() -> Vec<ModelInfo> {
    all_predefined_models()
        .into_iter()
        .filter(|m| matches!(m.provider, ModelProvider::MlxFunASR))
        .collect()
}

/// Check if an MLX model is available in the app's models directory.
/// Looks for any `.safetensors` file under the model's snapshots directory.
pub fn check_mlx_model_downloaded(model_repo: &str) -> bool {
    let cache_name = model_repo.replace('/', "--");
    let cache_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".voiceink/models")
        .join(format!("models--{}", cache_name))
        .join("snapshots");

    log::info!("[transcriber] check_mlx_model_downloaded repo={} path={:?} exists={}", model_repo, cache_path, cache_path.exists());

    if !cache_path.exists() {
        return false;
    }

    if let Ok(entries) = std::fs::read_dir(&cache_path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Ok(files) = std::fs::read_dir(&p) {
                    for file in files.flatten() {
                        let fp = file.path();
                        let ext = fp.extension().map(|e| e.to_string_lossy().to_string());
                        let is_symlink = fp.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false);
                        let target_exists = fp.exists(); // follows symlink
                        if ext.as_deref() == Some("safetensors") && target_exists {
                            log::info!("[transcriber] found safetensors: {:?} symlink={} target_exists={}", fp, is_symlink, target_exists);
                            return true;
                        }
                    }
                }
            }
        }
    }

    log::info!("[transcriber] no safetensors found for {}", model_repo);
    false
}

pub fn all_models(models_dir: &Path) -> Vec<ModelInfo> {
    let dirs = model_search_dirs(models_dir);
    let mut models = all_predefined_models();

    for model in &mut models {
        match model.provider {
            ModelProvider::Local => {
                model.is_downloaded = find_model_file(&dirs, &model.id);
            }
            ModelProvider::MlxWhisper | ModelProvider::MlxFunASR => {
                if let Some(repo) = &model.model_repo {
                    model.is_downloaded = check_mlx_model_downloaded(repo);
                }
            }
            _ => {
                // Cloud models: always "available" (no download needed)
                model.is_downloaded = true;
            }
        }
    }

    models
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

/// Directories where this app stores downloaded models.
fn model_search_dirs(app_models_dir: &Path) -> Vec<PathBuf> {
    vec![app_models_dir.to_path_buf()]
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

/// Transcribe audio via the MLX FunASR daemon (async, non-blocking).
/// Writes samples to a temp WAV file and sends the path to the daemon.
pub async fn transcribe_via_daemon(
    daemon: &crate::daemon::DaemonManager,
    samples: &[f32],
    sample_rate: u32,
    language: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<TranscriptionResult, AppError> {
    let start = std::time::Instant::now();

    // Write samples to a temp WAV file under ~/.voiceink (daemon expects a file path)
    let tmp_dir = dirs::home_dir().unwrap_or_default().join(".voiceink/tmp");
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
        "max_tokens": max_tokens,
        "temperature": temperature,
    });

    let timeout = std::time::Duration::from_secs(120);
    let resp = daemon.send_command_async(&cmd, timeout).await?;

    // Cleanup temp file
    let _ = std::fs::remove_file(&wav_path);

    // Check daemon memory after each transcription — restart if bloated
    daemon.check_and_restart_if_bloated();

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
    temperature: f32,
) -> Result<TranscriptionResult, AppError> {
    let mut params =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_temperature(temperature);
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
