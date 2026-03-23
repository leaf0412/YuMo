use std::sync::{Mutex, OnceLock};

use napi::bindgen_prelude::*;
use napi_derive::napi;
use yumo_core::daemon::DaemonManager;
use yumo_core::db;
use yumo_core::platform;
use yumo_core::state::{AppContext, AppPaths};
use yumo_core::{audio_io, text_processor, transcriber};

// ---------------------------------------------------------------------------
// Global state (initialized once via `init`)
// ---------------------------------------------------------------------------

static APP_CTX: OnceLock<AppContext> = OnceLock::new();
static DAEMON: OnceLock<Mutex<DaemonManager>> = OnceLock::new();

fn ctx() -> Result<&'static AppContext> {
    APP_CTX
        .get()
        .ok_or_else(|| Error::from_reason("AppContext not initialized — call init() first"))
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize the core with a data directory path.
/// Must be called once before any other function.
#[napi]
pub fn init(data_dir: String) -> Result<()> {
    // Initialize Rust logging so paster/core logs go to stderr (visible in Electron console)
    let _ = env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let mut paths = AppPaths::defaults();
    paths.data_dir = data_dir.clone().into();
    paths.models_dir = std::path::PathBuf::from(&data_dir).join("models");
    paths.sprites_dir = std::path::PathBuf::from(&data_dir).join("sprites");
    paths.recordings_dir = std::path::PathBuf::from(&data_dir).join("recordings");
    // Ensure data directory exists
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| Error::from_reason(format!("Failed to create data dir: {e}")))?;

    let db_path = std::path::PathBuf::from(&data_dir).join("data.db");
    let conn = db::init_database(&db_path)
        .map_err(|e| Error::from_reason(format!("Failed to init database: {e}")))?;

    let saved_settings = db::get_all_settings(&conn).unwrap_or_default();
    let app_ctx = AppContext::new(conn, paths, saved_settings);

    let daemon_script = std::path::PathBuf::from(&data_dir).join("mlx_funasr_daemon.py");
    let daemon = DaemonManager::new(daemon_script, data_dir.clone().into());

    // Cache audio devices at startup
    match platform::recorder::list_input_devices() {
        Ok(devices) => {
            log::info!("[startup] cached {} audio devices", devices.len());
            *app_ctx.device_cache.write().unwrap() = devices;
        }
        Err(e) => log::info!("[startup] device enumeration failed: {}", e),
    }

    APP_CTX
        .set(app_ctx)
        .map_err(|_| Error::from_reason("AppContext already initialized"))?;
    DAEMON
        .set(Mutex::new(daemon))
        .map_err(|_| Error::from_reason("Daemon already initialized"))?;

    // Pre-warm AudioUnit in background for fast first recording
    std::thread::spawn(|| {
        if let Ok(ctx) = ctx() {
            let dev_id = ctx.resolve_device_id();
            if dev_id > 0 {
                match platform::recorder::prepare_recording(dev_id) {
                    Ok(Some(prepared)) => {
                        log::info!("[startup] AudioUnit pre-warmed for device_id={}", dev_id);
                        *ctx.prepared_recording.lock().unwrap() = Some(prepared);
                    }
                    Ok(None) => log::info!("[startup] platform does not support pre-warming"),
                    Err(e) => log::info!("[startup] pre-warm failed: {}", e),
                }
            }
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Audio devices
// ---------------------------------------------------------------------------

#[napi(object)]
pub struct NapiAudioDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

/// List available audio input devices.
#[napi]
pub async fn list_audio_devices() -> Result<Vec<NapiAudioDevice>> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let devices = yumo_core::platform::recorder::list_input_devices()
            .map_err(|e| Error::from_reason(format!("Failed to list devices: {e}")))?;
        // Refresh device cache
        if let Ok(mut cache) = app.device_cache.write() {
            *cache = devices.clone();
        }
        Ok(devices
            .into_iter()
            .map(|d| NapiAudioDevice {
                id: d.id,
                name: d.name,
                is_default: d.is_default,
            })
            .collect())
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Get all settings as a JSON string.
#[napi]
pub async fn get_all_settings() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let settings = db::get_all_settings(&conn)
            .map_err(|e| Error::from_reason(format!("get_all_settings: {e}")))?;
        serde_json::to_string(&settings)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Transcriptions
// ---------------------------------------------------------------------------

/// Get transcriptions with cursor-based pagination.
/// Returns a JSON string of `{ items, next_cursor }`.
#[napi]
pub async fn get_transcriptions(
    cursor: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let lim = limit.unwrap_or(50) as usize;
        let result = db::get_transcriptions(
            &conn,
            cursor.as_deref(),
            query.as_deref(),
            lim,
        )
        .map_err(|e| Error::from_reason(format!("get_transcriptions: {e}")))?;
        serde_json::to_string(&result)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// List all available models (local + cloud).
#[napi]
pub async fn list_available_models() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let models = transcriber::all_models(&app.paths.models_dir);
        serde_json::to_string(&models)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Settings update
// ---------------------------------------------------------------------------

/// Update a single setting.
#[napi]
pub async fn update_setting(key: String, value: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let json_value: serde_json::Value = serde_json::from_str(&value)
            .unwrap_or_else(|_| serde_json::Value::String(value));
        app.set_setting_cached(&key, &json_value)
            .map_err(|e| Error::from_reason(format!("set_setting_cached: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Get transcription statistics.
#[napi]
pub async fn get_statistics(days: Option<u32>) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let stats = db::get_statistics(&conn, days.map(|d| d as i64))
            .map_err(|e| Error::from_reason(format!("get_statistics: {e}")))?;
        serde_json::to_string(&stats)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Vocabulary & Replacements
// ---------------------------------------------------------------------------

#[napi]
pub async fn get_vocabulary() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let words = db::get_vocabulary(&conn)
            .map_err(|e| Error::from_reason(format!("get_vocabulary: {e}")))?;
        serde_json::to_string(&words)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn add_vocabulary(word: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::add_vocabulary(&conn, &word)
            .map_err(|e| Error::from_reason(format!("add_vocabulary: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn delete_vocabulary(id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::delete_vocabulary(&conn, &id)
            .map_err(|e| Error::from_reason(format!("delete_vocabulary: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn get_replacements() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let items = db::get_replacements(&conn)
            .map_err(|e| Error::from_reason(format!("get_replacements: {e}")))?;
        serde_json::to_string(&items)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn set_replacement(original: String, replacement: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::set_replacement(&conn, &original, &replacement)
            .map_err(|e| Error::from_reason(format!("set_replacement: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn delete_replacement(id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::delete_replacement(&conn, &id)
            .map_err(|e| Error::from_reason(format!("delete_replacement: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Transcription CRUD
// ---------------------------------------------------------------------------

#[napi]
pub async fn delete_transcription(id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::delete_transcription(&conn, &id)
            .map_err(|e| Error::from_reason(format!("delete_transcription: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn delete_all_transcriptions() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::delete_all_transcriptions(&conn)
            .map_err(|e| Error::from_reason(format!("delete_all_transcriptions: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

#[napi]
pub async fn list_prompts() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let prompts = db::list_prompts(&conn)
            .map_err(|e| Error::from_reason(format!("list_prompts: {e}")))?;
        serde_json::to_string(&prompts)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn add_prompt(
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::add_prompt(&conn, &name, &system_msg, &user_msg, false)
            .map_err(|e| Error::from_reason(format!("add_prompt: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn update_prompt(
    id: String,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::update_prompt(&conn, &id, &name, &system_msg, &user_msg)
            .map_err(|e| Error::from_reason(format!("update_prompt: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn delete_prompt(id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::delete_prompt(&conn, &id)
            .map_err(|e| Error::from_reason(format!("delete_prompt: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// CSV Import / Export
// ---------------------------------------------------------------------------

#[napi]
pub async fn import_dictionary_csv(path: String, dict_type: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let p = std::path::Path::new(&path);
        match dict_type.as_str() {
            "vocabulary" => db::import_vocabulary_csv(&conn, p)
                .map_err(|e| Error::from_reason(format!("import_vocabulary_csv: {e}"))),
            "replacements" => db::import_replacements_csv(&conn, p)
                .map_err(|e| Error::from_reason(format!("import_replacements_csv: {e}"))),
            _ => Err(Error::from_reason(format!("Unknown dict type: {dict_type}"))),
        }
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn export_dictionary_csv(path: String, dict_type: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let p = std::path::Path::new(&path);
        match dict_type.as_str() {
            "vocabulary" => db::export_vocabulary_csv(&conn, p)
                .map_err(|e| Error::from_reason(format!("export_vocabulary_csv: {e}"))),
            "replacements" => db::export_replacements_csv(&conn, p)
                .map_err(|e| Error::from_reason(format!("export_replacements_csv: {e}"))),
            _ => Err(Error::from_reason(format!("Unknown dict type: {dict_type}"))),
        }
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Keychain
// ---------------------------------------------------------------------------

#[napi]
pub async fn store_api_key(provider: String, key: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        yumo_core::platform::keychain::store_key("com.voiceink.app", &provider, &key)
            .map_err(|e| Error::from_reason(format!("store_key: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn get_api_key(provider: String) -> Result<Option<String>> {
    tokio::task::spawn_blocking(move || {
        yumo_core::platform::keychain::get_key("com.voiceink.app", &provider)
            .map_err(|e| Error::from_reason(format!("get_key: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn delete_api_key(provider: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        yumo_core::platform::keychain::delete_key("com.voiceink.app", &provider)
            .map_err(|e| Error::from_reason(format!("delete_key: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Daemon management
// ---------------------------------------------------------------------------

fn daemon() -> Result<std::sync::MutexGuard<'static, DaemonManager>> {
    DAEMON
        .get()
        .ok_or_else(|| Error::from_reason("Daemon not initialized"))?
        .lock()
        .map_err(|e| Error::from_reason(format!("Daemon lock: {e}")))
}

#[napi(object)]
pub struct NapiDaemonStatus {
    pub running: bool,
    pub loaded_model: Option<String>,
}

#[napi]
pub fn daemon_status() -> Result<NapiDaemonStatus> {
    let d = daemon()?;
    Ok(NapiDaemonStatus {
        running: d.is_running(),
        loaded_model: d.loaded_model(),
    })
}

#[napi]
pub async fn daemon_start() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        let d = daemon()?;
        d.start().map_err(|e| Error::from_reason(format!("daemon start: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub fn daemon_stop() -> Result<()> {
    let d = daemon()?;
    d.stop();
    Ok(())
}

#[napi]
pub async fn daemon_load_model(model_repo: String) -> Result<()> {
    let repo = model_repo.clone();
    tokio::task::spawn_blocking(move || {
        let d = daemon()?;
        if !d.is_running() {
            d.start().map_err(|e| Error::from_reason(format!("daemon start: {e}")))?;
        }
        let cmd = serde_json::json!({"action": "load", "model": repo});
        let resp = d.send_command(&cmd)
            .map_err(|e| Error::from_reason(format!("daemon load: {e}")))?;
        if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" {
            d.set_loaded_model(Some(model_repo));
            Ok(())
        } else {
            Err(Error::from_reason(resp.error.unwrap_or_else(|| format!("load failed: {}", resp.status))))
        }
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn daemon_unload_model() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        let d = daemon()?;
        let cmd = serde_json::json!({"action": "unload"});
        d.send_command(&cmd)
            .map_err(|e| Error::from_reason(format!("daemon unload: {e}")))?;
        d.set_loaded_model(None);
        Ok(())
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

#[napi]
pub async fn daemon_check_deps() -> Result<bool> {
    tokio::task::spawn_blocking(|| {
        let d = daemon()?;
        Ok(d.has_python())
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Recording pipeline
// ---------------------------------------------------------------------------

/// Start recording from the given audio device (or default).
/// Kept synchronous: creates a RecordingHandle that must stay on the napi thread.
#[napi]
pub fn start_recording(device_id: Option<u32>) -> Result<String> {
    let app = ctx()?;

    // 1. Check idle
    {
        let pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        if *pipeline != yumo_core::pipeline::PipelineState::Idle {
            return Err(Error::from_reason("Already recording"));
        }
    }

    // 2. Read settings from cache and handle system mute
    let settings = app.settings_cache.read()
        .map_err(|e| Error::from_reason(format!("settings lock: {e}")))?
        .clone();
    let mute = settings.get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if mute {
        let _ = platform::audio_ctrl::set_system_muted(true);
    }

    // 3. Resolve device from cache (fallback to fresh enumeration)
    let devices = {
        let cached = app.device_cache.read()
            .map_err(|e| Error::from_reason(format!("device cache lock: {e}")))?;
        if cached.is_empty() {
            drop(cached);
            let fresh = platform::recorder::list_input_devices()
                .map_err(|e| Error::from_reason(format!("list devices: {e}")))?;
            if let Ok(mut cache) = app.device_cache.write() {
                *cache = fresh.clone();
            }
            fresh
        } else {
            cached.clone()
        }
    };
    if devices.is_empty() {
        return Err(Error::from_reason("No input devices found"));
    }
    let dev_id = device_id.unwrap_or_else(|| {
        let saved = settings.get("audio_device")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        if let Some(id) = saved {
            if devices.iter().any(|d| d.id == id) {
                return id;
            }
        }
        devices.iter().find(|d| d.is_default).map(|d| d.id).unwrap_or(devices[0].id)
    });

    // 4. Start recording (try prepared path first)
    let (handle, _level_rx) = {
        let mut prepared_slot = app.prepared_recording.lock()
            .map_err(|e| Error::from_reason(format!("prepared lock: {e}")))?;
        if let Some(prepared) = prepared_slot.take() {
            if prepared.device_id == dev_id {
                log::info!("[pipeline] using prepared recording for device_id={}", dev_id);
                match platform::recorder::start_prepared_recording(prepared) {
                    Ok(result) => result,
                    Err(e) => {
                        log::warn!("[pipeline] start_prepared failed: {}, cold start", e);
                        platform::recorder::start_recording(dev_id)
                            .map_err(|e| Error::from_reason(format!("start_recording: {e}")))?
                    }
                }
            } else {
                log::info!("[pipeline] device mismatch, cold start");
                drop(prepared);
                platform::recorder::start_recording(dev_id)
                    .map_err(|e| Error::from_reason(format!("start_recording: {e}")))?
            }
        } else {
            log::info!("[pipeline] no prepared recording, cold start");
            platform::recorder::start_recording(dev_id)
                .map_err(|e| Error::from_reason(format!("start_recording: {e}")))?
        }
    };

    // 5. Store handle and update state
    {
        let mut rec = app.recording_handle.lock()
            .map_err(|e| Error::from_reason(format!("recording lock: {e}")))?;
        *rec = Some(handle);
    }
    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Recording;
    }

    Ok(serde_json::json!({"state": "recording"}).to_string())
}

/// Stop recording, transcribe, apply text processing, paste, and save to DB.
/// Returns a JSON string with the transcription result.
#[napi]
pub async fn stop_recording() -> Result<String> {
    let app = ctx()?;

    // 1. Take recording handle
    let handle = {
        app.recording_handle.lock()
            .map_err(|e| Error::from_reason(format!("recording lock: {e}")))?
            .take()
            .ok_or_else(|| Error::from_reason("Not recording"))?
    };

    // 1.5 Update state to Processing
    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Processing;
    }

    // 2. Stop recording and get audio data
    let audio_data = platform::recorder::stop_recording(handle)
        .map_err(|e| Error::from_reason(format!("stop_recording: {e}")))?;

    let audio_duration_secs = if audio_data.sample_rate > 0 && audio_data.channels > 0 {
        audio_data.pcm_samples.len() as f64 / audio_data.sample_rate as f64 / audio_data.channels as f64
    } else {
        0.0
    };

    // 2.5 Save recording WAV
    let recording_path = audio_io::save_recording(&audio_data, &app.paths.recordings_dir)
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // 3. Update state to Transcribing
    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Transcribing;
    }

    // 4. Read settings
    let settings_map = {
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::get_all_settings(&conn)
            .map_err(|e| Error::from_reason(format!("get_all_settings: {e}")))?
    };
    let model_id = settings_map.get("selected_model_id")
        .and_then(|v| v.as_str())
        .unwrap_or("ggml-base.en")
        .to_string();
    let language = settings_map.get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_string();
    let temperature_key = format!("model_{}_temperature", model_id);
    let max_tokens_key = format!("model_{}_max_tokens", model_id);
    let temperature: f64 = settings_map.get(&temperature_key)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let max_tokens: u32 = settings_map.get(&max_tokens_key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(1900);

    // 5. Transcribe — route by model provider
    let all = transcriber::all_models(&app.paths.models_dir);
    let model_info = all.iter().find(|m| m.id == model_id);

    let transcribe_result = match model_info.map(|m| &m.provider) {
        Some(transcriber::ModelProvider::MlxFunASR) => {
            // Use daemon for MLX transcription
            let d = daemon()?;
            if !d.is_running() {
                d.start().map_err(|e| Error::from_reason(format!("daemon start: {e}")))?;
            }
            // Auto-load model if needed
            let model_repo = model_info.and_then(|m| m.model_repo.clone());
            let loaded = d.loaded_model();
            if model_repo.is_some() && loaded.as_ref() != model_repo.as_ref() {
                let repo = model_repo.as_ref().unwrap();
                let cmd = serde_json::json!({"action": "load", "model": repo});
                let resp = d.send_command(&cmd)
                    .map_err(|e| Error::from_reason(format!("daemon load: {e}")))?;
                if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" {
                    d.set_loaded_model(model_repo.clone());
                } else {
                    let msg = resp.error.unwrap_or_else(|| format!("load failed: {}", resp.status));
                    // Reset to idle on failure
                    let mut pipeline = app.pipeline_state.lock()
                        .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
                    *pipeline = yumo_core::pipeline::PipelineState::Idle;
                    return Err(Error::from_reason(msg));
                }
            }
            // Write temp WAV and send to daemon synchronously
            let samples = audio_data.pcm_samples.clone();
            let sr = audio_data.sample_rate;
            let lang = language.clone();
            let tmp_dir = dirs::home_dir().unwrap_or_default().join(".voiceink/tmp");
            std::fs::create_dir_all(&tmp_dir)
                .map_err(|e| Error::from_reason(format!("create tmp dir: {e}")))?;
            let wav_path = tmp_dir.join("recording.wav");
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: sr,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut writer = hound::WavWriter::create(&wav_path, spec)
                .map_err(|e| Error::from_reason(format!("wav create: {e}")))?;
            for &s in &samples {
                writer.write_sample(s).map_err(|e| Error::from_reason(format!("wav write: {e}")))?;
            }
            writer.finalize().map_err(|e| Error::from_reason(format!("wav finalize: {e}")))?;

            let cmd = serde_json::json!({
                "action": "transcribe",
                "audio": wav_path.to_string_lossy(),
                "language": lang,
                "max_tokens": max_tokens,
                "temperature": temperature,
            });
            // Must drop previous guard, re-acquire for send_command
            drop(d);
            let daemon_ref = daemon()?;
            let resp = daemon_ref.send_command(&cmd)
                .map_err(|e| Error::from_reason(format!("daemon transcribe: {e}")))?;
            let _ = std::fs::remove_file(&wav_path);
            daemon_ref.check_and_restart_if_bloated();

            if resp.status == "success" {
                Ok(transcriber::TranscriptionResult {
                    text: resp.text.unwrap_or_default(),
                    duration_ms: 0,
                })
            } else {
                Err(Error::from_reason(
                    resp.error.unwrap_or_else(|| "Transcription failed".into())
                ))
            }
        }
        _ => {
            // Local whisper model
            let model_path = transcriber::model_path(&app.paths.models_dir, &model_id);
            if !model_path.exists() {
                let mut pipeline = app.pipeline_state.lock()
                    .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
                *pipeline = yumo_core::pipeline::PipelineState::Idle;
                return Err(Error::from_reason("No model downloaded"));
            }
            let samples = audio_data.pcm_samples.clone();
            let sr = audio_data.sample_rate;
            let lang = language.clone();
            let temp = temperature as f32;
            // Run whisper in a blocking task
            tokio::task::spawn_blocking(move || {
                let wctx = transcriber::load_model(&model_path)
                    .map_err(|e| Error::from_reason(format!("load_model: {e}")))?;
                transcriber::transcribe(&wctx, &samples, sr, &lang, temp)
                    .map_err(|e| Error::from_reason(format!("transcribe: {e}")))
            })
            .await
            .map_err(|e| Error::from_reason(format!("spawn_blocking: {e}")))?
        }
    };

    // Handle transcription failure
    let result = match transcribe_result {
        Ok(r) => r,
        Err(e) => {
            let mut pipeline = app.pipeline_state.lock()
                .map_err(|er| Error::from_reason(format!("pipeline lock: {er}")))?;
            *pipeline = yumo_core::pipeline::PipelineState::Idle;
            return Err(e);
        }
    };
    let text = result.text;

    // 6. Apply text processing
    let replacements: Vec<(String, String)> = {
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::get_replacements(&conn)
            .map_err(|e| Error::from_reason(format!("get_replacements: {e}")))?
            .into_iter()
            .map(|r| (r.original, r.replacement))
            .collect()
    };
    let auto_capitalize = settings_map.get("auto_capitalize")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let processed_text = text_processor::process_text(&text, &replacements, auto_capitalize);

    // 7. Paste
    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Pasting;
    }

    let final_text = &processed_text;

    let paste_error: Option<String> = if platform::permissions::check_accessibility() {
        let restore_delay = settings_map.get("clipboard_restore_delay")
            .and_then(|v| v.as_f64())
            .unwrap_or(1500.0) as u64;
        #[cfg(target_os = "linux")]
        {
            platform::paster::paste_text(final_text, restore_delay)
                .err()
                .map(|e| e.to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            platform::paster::paste_text(final_text, restore_delay);
            None
        }
    } else {
        platform::paster::write_clipboard(final_text);
        None
    };

    // 8. Save to DB
    let word_count = db::count_words(final_text) as i32;
    {
        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        db::insert_transcription(
            &conn,
            &processed_text,
            None, // enhanced_text not implemented yet
            audio_duration_secs,
            &model_id,
            word_count,
            recording_path.as_deref(),
        )
        .map_err(|e| Error::from_reason(format!("insert_transcription: {e}")))?;
    }

    // 8.1 Save transcription text alongside WAV
    if let Some(ref wav_path) = recording_path {
        let txt_path = std::path::Path::new(wav_path).with_extension("txt");
        let _ = std::fs::write(&txt_path, final_text);
    }

    // 9. Unmute system audio
    if settings_map.get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let _ = platform::audio_ctrl::set_system_muted(false);
    }

    // 10. Back to idle
    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Idle;
    }

    // Background re-prepare for next recording
    std::thread::spawn(move || {
        if let Ok(ctx) = ctx() {
            let dev_id = ctx.resolve_device_id();
            if dev_id > 0 {
                match platform::recorder::prepare_recording(dev_id) {
                    Ok(Some(prepared)) => {
                        log::info!("[pipeline] background prepare complete for device_id={}", dev_id);
                        *ctx.prepared_recording.lock().unwrap() = Some(prepared);
                    }
                    Ok(None) => log::info!("[pipeline] platform does not support pre-warming"),
                    Err(e) => log::warn!("[pipeline] background prepare failed: {}", e),
                }
            }
        }
    });

    let result_json = serde_json::json!({
        "text": processed_text,
        "enhanced_text": serde_json::Value::Null,
        "paste_error": paste_error,
    });
    Ok(result_json.to_string())
}

/// Cancel an in-progress recording, discarding audio.
#[napi]
pub fn cancel_recording() -> Result<()> {
    let app = ctx()?;

    let handle = app.recording_handle.lock()
        .map_err(|e| Error::from_reason(format!("recording lock: {e}")))?
        .take();
    if let Some(h) = handle {
        let _ = platform::recorder::cancel_recording(h);
    }

    {
        let mut pipeline = app.pipeline_state.lock()
            .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
        *pipeline = yumo_core::pipeline::PipelineState::Idle;
    }

    // Unmute if needed
    if let Ok(conn) = app.db.lock() {
        if let Ok(settings) = db::get_all_settings(&conn) {
            if settings.get("system_mute_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let _ = platform::audio_ctrl::set_system_muted(false);
            }
        }
    }

    // Background re-prepare for next recording
    std::thread::spawn(move || {
        if let Ok(ctx) = ctx() {
            let dev_id = ctx.resolve_device_id();
            if dev_id > 0 {
                match platform::recorder::prepare_recording(dev_id) {
                    Ok(Some(prepared)) => {
                        log::info!("[pipeline] background prepare complete for device_id={}", dev_id);
                        *ctx.prepared_recording.lock().unwrap() = Some(prepared);
                    }
                    Ok(None) => log::info!("[pipeline] platform does not support pre-warming"),
                    Err(e) => log::warn!("[pipeline] background prepare failed: {}", e),
                }
            }
        }
    });

    Ok(())
}

/// Get current pipeline state as a JSON string.
#[napi]
pub fn get_pipeline_state() -> Result<String> {
    let app = ctx()?;
    let pipeline = app.pipeline_state.lock()
        .map_err(|e| Error::from_reason(format!("pipeline lock: {e}")))?;
    let s = match *pipeline {
        yumo_core::pipeline::PipelineState::Idle => "idle",
        yumo_core::pipeline::PipelineState::Recording => "recording",
        yumo_core::pipeline::PipelineState::Processing => "processing",
        yumo_core::pipeline::PipelineState::Transcribing => "transcribing",
        yumo_core::pipeline::PipelineState::Enhancing => "enhancing",
        yumo_core::pipeline::PipelineState::Pasting => "pasting",
    };
    Ok(serde_json::json!({"state": s}).to_string())
}

// ---------------------------------------------------------------------------
// Recording playback
// ---------------------------------------------------------------------------

/// Read a recording WAV file and return it as a base64 data URI.
#[napi]
pub async fn get_recording(recording_path: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let path = std::path::Path::new(&recording_path);
        audio_io::read_recording_as_data_uri(path)
            .map_err(|e| Error::from_reason(format!("read_recording: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Model download / delete
// ---------------------------------------------------------------------------

/// Download a whisper model by ID. Returns progress via polling (not streaming).
/// For simplicity, this blocks until download completes.
#[napi]
pub async fn download_model(model_id: String) -> Result<()> {
    let app = ctx()?;
    let models = transcriber::predefined_models();
    let model = models.iter().find(|m| m.id == model_id)
        .ok_or_else(|| Error::from_reason(format!("Model {} not found", model_id)))?;

    let dest = transcriber::model_path(&app.paths.models_dir, &model_id);
    std::fs::create_dir_all(&app.paths.models_dir)
        .map_err(|e| Error::from_reason(format!("create models dir: {e}")))?;

    let url = model.download_url.clone();
    yumo_core::downloader::download_file(&url, &dest, None).await
        .map_err(|e| Error::from_reason(format!("download: {e}")))?;

    Ok(())
}

/// Delete a model by ID (local whisper .bin or MLX cache directory).
#[napi]
pub async fn delete_model(model_id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let all = transcriber::all_models(&app.paths.models_dir);
        let model_info = all.iter().find(|m| m.id == model_id);

        if let Some(model) = model_info {
            match model.provider {
                transcriber::ModelProvider::MlxWhisper | transcriber::ModelProvider::MlxFunASR => {
                    // Delete HuggingFace cache directory
                    if let Some(repo) = &model.model_repo {
                        let cache_name = repo.replace('/', "--");
                        let cache_dir = app.paths.models_dir.join(format!("models--{}", cache_name));
                        if cache_dir.exists() {
                            std::fs::remove_dir_all(&cache_dir)
                                .map_err(|e| Error::from_reason(format!("remove MLX cache: {e}")))?;
                        }
                    }
                    // Unload if this model is currently loaded in daemon
                    if let Ok(d) = daemon() {
                        if d.loaded_model().as_deref() == model.model_repo.as_deref() {
                            let cmd = serde_json::json!({"action": "unload"});
                            let _ = d.send_command(&cmd);
                            d.set_loaded_model(None);
                        }
                    }
                }
                _ => {
                    let path = transcriber::model_path(&app.paths.models_dir, &model_id);
                    if path.exists() {
                        std::fs::remove_file(&path)
                            .map_err(|e| Error::from_reason(format!("remove model: {e}")))?;
                    }
                }
            }
        }

        // Clear selected_model_id if this was the selected model
        {
            let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
            let selected = db::get_setting(&conn, "selected_model_id")
                .map_err(|e| Error::from_reason(format!("get_setting: {e}")))?;
            if selected.as_ref().and_then(|v| v.as_str()) == Some(&model_id) {
                drop(conn);
                app.set_setting_cached("selected_model_id", &serde_json::json!(""))
                    .map_err(|e| Error::from_reason(format!("set_setting_cached: {e}")))?;
            }
        }

        Ok(())
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

// ---------------------------------------------------------------------------
// Sprites
// ---------------------------------------------------------------------------

/// List all available sprite sheet manifests as a JSON array.
#[napi]
pub async fn list_sprites() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let app = ctx()?;
        let base = &app.paths.sprites_dir;
        if !base.exists() {
            return Ok("[]".to_string());
        }
        let mut results = Vec::new();
        let entries = std::fs::read_dir(base)
            .map_err(|e| Error::from_reason(format!("read sprites dir: {e}")))?;
        for entry in entries {
            let entry = entry.map_err(|e| Error::from_reason(format!("read entry: {e}")))?;
            if !entry.path().is_dir() { continue; }
            let manifest_path = entry.path().join("manifest.json");
            if manifest_path.exists() {
                let data = std::fs::read_to_string(&manifest_path)
                    .map_err(|e| Error::from_reason(format!("read manifest: {e}")))?;
                if let Ok(mut manifest) = serde_json::from_str::<serde_json::Value>(&data) {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    manifest.as_object_mut().map(|m| {
                        m.insert("dirId".into(), serde_json::Value::String(dir_name));
                    });
                    results.push(manifest);
                }
            }
        }
        serde_json::to_string(&results)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Read a sprite sheet image file as base64 data URI.
#[napi]
pub async fn get_sprite_image(dir_id: String, file_name: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let path = app.paths.sprites_dir.join(&dir_id).join(&file_name);
        if !path.exists() {
            // Try processed version
            let processed = app.paths.sprites_dir.join(&dir_id).join("sprite_processed.png");
            if processed.exists() {
                let data = std::fs::read(&processed)
                    .map_err(|e| Error::from_reason(format!("read processed sprite: {e}")))?;
                let b64 = audio_io::base64_encode(&data);
                return Ok(format!("data:image/png;base64,{}", b64));
            }
            return Err(Error::from_reason(format!("Sprite image not found: {}", path.display())));
        }
        let data = std::fs::read(&path)
            .map_err(|e| Error::from_reason(format!("read sprite: {e}")))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let mime = match ext {
            "gif" => "image/gif",
            "jpg" | "jpeg" => "image/jpeg",
            _ => "image/png",
        };
        let b64 = audio_io::base64_encode(&data);
        Ok(format!("data:{};base64,{}", mime, b64))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Import a sprite from a folder path. The folder must contain manifest.json.
/// Returns the manifest JSON with dirId added.
#[napi]
pub async fn import_sprite_folder(path: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let dir_path = std::path::Path::new(&path);
        let manifest_path = dir_path.join("manifest.json");
        if !manifest_path.exists() {
            return Err(Error::from_reason("manifest.json not found in folder"));
        }

        let manifest_data = std::fs::read_to_string(&manifest_path)
            .map_err(|e| Error::from_reason(format!("read manifest: {e}")))?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_data)
            .map_err(|e| Error::from_reason(format!("parse manifest: {e}")))?;

        let sprite_file = manifest.get("spriteFile")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("manifest.json missing spriteFile"))?;

        let sprite_path = dir_path.join(sprite_file);
        if !sprite_path.exists() {
            return Err(Error::from_reason(format!("Sprite file not found: {}", sprite_file)));
        }

        // Create destination
        let dest_id = uuid::Uuid::new_v4().to_string().to_uppercase();
        let dest_dir = app.paths.sprites_dir.join(&dest_id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| Error::from_reason(format!("create sprite dir: {e}")))?;

        std::fs::copy(&manifest_path, dest_dir.join("manifest.json"))
            .map_err(|e| Error::from_reason(format!("copy manifest: {e}")))?;
        std::fs::copy(&sprite_path, dest_dir.join(sprite_file))
            .map_err(|e| Error::from_reason(format!("copy sprite: {e}")))?;

        // Copy processed version if exists
        let processed = dir_path.join("sprite_processed.png");
        if processed.exists() {
            let _ = std::fs::copy(&processed, dest_dir.join("sprite_processed.png"));
        }

        let mut result = manifest;
        result.as_object_mut().map(|m| {
            m.insert("dirId".into(), serde_json::Value::String(dest_id));
        });
        serde_json::to_string(&result)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Import a sprite from a .zip archive path.
/// Returns the manifest JSON with dirId added.
#[napi]
pub async fn import_sprite_zip(zip_path: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let file_path = std::path::Path::new(&zip_path);

        // Extract to temp directory
        let tmp_dir = std::env::temp_dir().join(format!("sprite-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| Error::from_reason(format!("create tmp dir: {e}")))?;

        let zip_file = std::fs::File::open(file_path)
            .map_err(|e| Error::from_reason(format!("open zip: {e}")))?;
        let mut archive = zip::ZipArchive::new(zip_file)
            .map_err(|e| Error::from_reason(format!("read zip: {e}")))?;
        archive.extract(&tmp_dir)
            .map_err(|e| Error::from_reason(format!("extract zip: {e}")))?;

        // Find manifest.json recursively
        let manifest_path = find_manifest_in_dir(&tmp_dir)
            .ok_or_else(|| Error::from_reason("manifest.json not found in zip"))?;

        let manifest_data = std::fs::read_to_string(&manifest_path)
            .map_err(|e| Error::from_reason(format!("read manifest: {e}")))?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_data)
            .map_err(|e| Error::from_reason(format!("parse manifest: {e}")))?;

        let sprite_file = manifest.get("spriteFile")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("manifest.json missing spriteFile"))?;

        let manifest_dir = manifest_path.parent().unwrap_or(&tmp_dir);
        let sprite_path = manifest_dir.join(sprite_file);
        if !sprite_path.exists() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(Error::from_reason(format!("Sprite file not found: {}", sprite_file)));
        }

        // Create destination
        let dest_id = uuid::Uuid::new_v4().to_string().to_uppercase();
        let dest_dir = app.paths.sprites_dir.join(&dest_id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| Error::from_reason(format!("create sprite dir: {e}")))?;

        std::fs::copy(&manifest_path, dest_dir.join("manifest.json"))
            .map_err(|e| Error::from_reason(format!("copy manifest: {e}")))?;
        std::fs::copy(&sprite_path, dest_dir.join(sprite_file))
            .map_err(|e| Error::from_reason(format!("copy sprite: {e}")))?;

        let processed = manifest_dir.join("sprite_processed.png");
        if processed.exists() {
            let _ = std::fs::copy(&processed, dest_dir.join("sprite_processed.png"));
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);

        let mut result = manifest;
        result.as_object_mut().map(|m| {
            m.insert("dirId".into(), serde_json::Value::String(dest_id));
        });
        serde_json::to_string(&result)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Delete a sprite sheet by its directory ID.
#[napi]
pub async fn delete_sprite(dir_id: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let dir = app.paths.sprites_dir.join(&dir_id);
        if dir.exists() && dir.is_dir() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| Error::from_reason(format!("remove sprite dir: {e}")))?;
        }
        // Clear selected_sprite_id if it was the deleted one
        {
            let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
            if let Ok(Some(current)) = db::get_setting(&conn, "selected_sprite_id") {
                if current.as_str() == Some(dir_id.as_str()) {
                    drop(conn);
                    let _ = app.set_setting_cached("selected_sprite_id", &serde_json::Value::String(String::new()));
                }
            }
        }
        Ok(())
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Process a sprite sheet image: remove background color and save as sprite_processed.png.
/// Mirrors Tauri commands.rs process_sprite_background logic.
#[napi]
pub async fn process_sprite_background(dir_id: String, threshold: f64) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let dir = app.paths.sprites_dir.join(&dir_id);
        let manifest_path = dir.join("manifest.json");
        let manifest_data = std::fs::read_to_string(&manifest_path)
            .map_err(|e| Error::from_reason(format!("read manifest: {e}")))?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_data)
            .map_err(|e| Error::from_reason(format!("manifest.json parse error: {e}")))?;

        let sprite_file = manifest
            .get("spriteFile")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("missing spriteFile"))?;

        let src_path = dir.join(sprite_file);
        let mut img = image::open(&src_path)
            .map_err(|e| Error::from_reason(format!("failed to open image: {e}")))?
            .to_rgba8();

        let (width, height) = img.dimensions();

        // Sample 4 corners (inset 14px)
        let inset = 14u32.min(width.min(height) / 4);
        let corners = [
            (inset, inset),
            (width - 1 - inset, inset),
            (inset, height - 1 - inset),
            (width - 1 - inset, height - 1 - inset),
        ];

        let samples: Vec<(f64, f64, f64)> = corners
            .iter()
            .map(|&(cx, cy)| {
                let p = img.get_pixel(cx, cy);
                (p[0] as f64 / 255.0, p[1] as f64 / 255.0, p[2] as f64 / 255.0)
            })
            .collect();

        // Pick two most similar corner colors
        let mut best_dist = f64::MAX;
        let (mut bi, mut bj) = (0usize, 1usize);
        for i in 0..4 {
            for j in (i + 1)..4 {
                let d = sprite_color_dist(&samples[i], &samples[j]);
                if d < best_dist {
                    best_dist = d;
                    bi = i;
                    bj = j;
                }
            }
        }

        let bg = (
            (samples[bi].0 + samples[bj].0) / 2.0,
            (samples[bi].1 + samples[bj].1) / 2.0,
            (samples[bi].2 + samples[bj].2) / 2.0,
        );

        // Replace matching pixels with transparent
        for pixel in img.pixels_mut() {
            let c = (
                pixel[0] as f64 / 255.0,
                pixel[1] as f64 / 255.0,
                pixel[2] as f64 / 255.0,
            );
            if sprite_color_dist(&c, &bg) < threshold {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
        }

        let dest_path = dir.join("sprite_processed.png");
        img.save(&dest_path)
            .map_err(|e| Error::from_reason(format!("failed to save processed image: {e}")))?;

        Ok(())
    })
    .await
    .map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

fn sprite_color_dist(a: &(f64, f64, f64), b: &(f64, f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// Check microphone and accessibility permissions (macOS-native checks).
#[napi]
pub async fn check_permissions() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        let status = platform::permissions::check_all();
        serde_json::to_string(&status)
            .map_err(|e| Error::from_reason(format!("JSON: {e}")))
    })
    .await
    .map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Open system settings for a specific permission type.
#[napi]
pub fn request_permission(permission_type: String) {
    match permission_type.as_str() {
        "microphone" => platform::permissions::open_microphone_settings(),
        "accessibility" => platform::permissions::open_accessibility_settings(),
        _ => {}
    }
}

/// Recursively find manifest.json in a directory.
fn find_manifest_in_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let direct = dir.join("manifest.json");
    if direct.exists() {
        return Some(direct);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_manifest_in_dir(&path) {
                    return Some(found);
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Legacy import
// ---------------------------------------------------------------------------

/// Auto-detect VoiceInk macOS data directory.
#[napi]
pub async fn detect_voiceink_legacy_path() -> Result<Option<String>> {
    tokio::task::spawn_blocking(|| {
        let home = dirs::home_dir();
        match home {
            Some(home) => {
                let store = home.join("Library/Application Support/com.prakashjoshipax.VoiceInk/default.store");
                if store.exists() {
                    Ok(Some(store.to_string_lossy().to_string()))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}

/// Import legacy VoiceInk data from a .store file path.
/// Returns a JSON string with import results.
#[napi]
pub async fn import_voiceink_legacy(store_path: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let app = ctx()?;
        let store = std::path::Path::new(&store_path);
        if !store.exists() {
            return Err(Error::from_reason("VoiceInk database file not found"));
        }

        let dict_store = store.parent().map(|p| p.join("dictionary.store"));
        let dict_ref = dict_store.as_deref().filter(|p| p.exists());

        let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
        let result = db::import_voiceink_legacy(&conn, store, dict_ref, &app.paths.recordings_dir)
            .map_err(|e| Error::from_reason(format!("import_legacy: {e}")))?;
        serde_json::to_string(&result)
            .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
    }).await.map_err(|e| Error::from_reason(format!("spawn: {e}")))?
}
