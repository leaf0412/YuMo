use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

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

        Self { data_dir, models_dir, sprites_dir, recordings_dir }
    }

    /// Defaults (no DB needed, for bootstrap before DB exists).
    pub fn defaults() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let data_dir = home.join(".voiceink");
        Self {
            models_dir: data_dir.join("models"),
            sprites_dir: data_dir.join("sprites"),
            recordings_dir: data_dir.join("recordings"),
            data_dir,
        }
    }
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub pipeline_state: Mutex<PipelineState>,
    pub recording_handle: Mutex<Option<RecordingHandle>>,
    pub paths: AppPaths,
    pub daemon: crate::daemon::DaemonManager,
}

impl AppState {
    pub fn new(conn: Connection, paths: AppPaths, daemon: crate::daemon::DaemonManager) -> Self {
        Self {
            db: Mutex::new(conn),
            pipeline_state: Mutex::new(PipelineState::Idle),
            recording_handle: Mutex::new(None),
            paths,
            daemon,
        }
    }
}
