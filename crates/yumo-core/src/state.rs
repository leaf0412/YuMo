use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::denoiser::DtlnDenoiser;
use crate::pipeline::PipelineState;
use crate::recorder::RecordingHandle;

/// All configurable paths with defaults.
/// Each can be overridden via DB setting `path_<name>`.
pub struct AppPaths {
    /// Root data dir (db, logs, daemon script)
    pub data_dir: PathBuf,
    /// Whisper model files
    pub models_dir: PathBuf,
    /// Sprite sheet assets
    pub sprites_dir: PathBuf,
    /// Saved recording WAV files
    pub recordings_dir: PathBuf,
    /// DTLN denoiser ONNX model files
    pub denoiser_dir: PathBuf,
}

impl AppPaths {
    /// Build from DB settings, falling back to defaults.
    pub fn from_settings(settings: &std::collections::HashMap<String, serde_json::Value>) -> Self {
        let home = dirs::home_dir().unwrap_or_default();

        let data_dir = settings
            .get("path_data")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".voiceink"));

        let models_dir = settings
            .get("path_models")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("models"));

        let sprites_dir = settings
            .get("path_sprites")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("sprites"));

        let recordings_dir = settings
            .get("path_recordings")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("recordings"));

        let denoiser_dir = settings
            .get("path_denoiser")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("denoiser"));

        Self { data_dir, models_dir, sprites_dir, recordings_dir, denoiser_dir }
    }

    /// Defaults (no DB needed, for bootstrap before DB exists).
    pub fn defaults() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let data_dir = home.join(".voiceink");
        Self {
            models_dir: data_dir.join("models"),
            sprites_dir: data_dir.join("sprites"),
            recordings_dir: data_dir.join("recordings"),
            denoiser_dir: data_dir.join("denoiser"),
            data_dir,
        }
    }
}

/// Core application context (platform-agnostic).
///
/// The `daemon` (DaemonManager) is managed as a separate Tauri State
/// to avoid circular locking with the process mutex.
pub struct AppContext {
    pub db: Mutex<Connection>,
    pub pipeline_state: Mutex<PipelineState>,
    pub recording_handle: Mutex<Option<RecordingHandle>>,
    pub paths: AppPaths,
    pub denoiser: Mutex<Option<DtlnDenoiser>>,
}

impl AppContext {
    pub fn new(conn: Connection, paths: AppPaths) -> Self {
        Self {
            db: Mutex::new(conn),
            pipeline_state: Mutex::new(PipelineState::Idle),
            recording_handle: Mutex::new(None),
            paths,
            denoiser: Mutex::new(None),
        }
    }
}
