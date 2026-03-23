use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use crate::pipeline::PipelineState;
use crate::platform::{AudioInputDevice, RecordingHandle};

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

/// Core application context (platform-agnostic).
///
/// The `daemon` (DaemonManager) is managed as a separate Tauri State
/// to avoid circular locking with the process mutex.
pub struct AppContext {
    pub db: Mutex<Connection>,
    pub pipeline_state: Mutex<PipelineState>,
    pub recording_handle: Mutex<Option<RecordingHandle>>,
    pub paths: AppPaths,
    pub settings_cache: RwLock<HashMap<String, Value>>,
    pub device_cache: RwLock<Vec<AudioInputDevice>>,
}

impl AppContext {
    pub fn new(conn: Connection, paths: AppPaths, initial_settings: HashMap<String, Value>) -> Self {
        Self {
            db: Mutex::new(conn),
            pipeline_state: Mutex::new(PipelineState::Idle),
            recording_handle: Mutex::new(None),
            paths,
            settings_cache: RwLock::new(initial_settings),
            device_cache: RwLock::new(Vec::new()),
        }
    }

    /// Write a setting to both the DB and the in-memory cache atomically.
    pub fn set_setting_cached(&self, key: &str, value: &Value) -> Result<(), crate::error::AppError> {
        {
            let conn = self.db.lock()
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            crate::db::update_setting(&conn, key, value)?;
        }
        {
            let mut cache = self.settings_cache.write()
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            cache.insert(key.to_string(), value.clone());
        }
        Ok(())
    }

    /// Resolve the target audio device ID from cache.
    ///
    /// Priority: explicit `device_id` > saved `audio_device` setting > default device > 0.
    pub fn resolve_device_id(&self) -> u32 {
        let settings = self.settings_cache.read().unwrap_or_else(|e| e.into_inner());
        let devices = self.device_cache.read().unwrap_or_else(|e| e.into_inner());
        let saved = settings.get("audio_device")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        if let Some(id) = saved {
            if devices.iter().any(|d| d.id == id) {
                return id;
            }
        }
        devices.iter().find(|d| d.is_default).map(|d| d.id).unwrap_or(0)
    }
}
