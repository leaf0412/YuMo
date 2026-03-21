use std::sync::{Mutex, OnceLock};

use napi::bindgen_prelude::*;
use napi_derive::napi;
use yumo_core::daemon::DaemonManager;
use yumo_core::db;
use yumo_core::state::{AppContext, AppPaths};
use yumo_core::transcriber;

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

    let app_ctx = AppContext::new(conn, paths);

    let daemon_script = std::path::PathBuf::from(&data_dir).join("mlx_funasr_daemon.py");
    let daemon = DaemonManager::new(daemon_script, data_dir.clone().into());

    APP_CTX
        .set(app_ctx)
        .map_err(|_| Error::from_reason("AppContext already initialized"))?;
    DAEMON
        .set(Mutex::new(daemon))
        .map_err(|_| Error::from_reason("Daemon already initialized"))?;

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
pub fn list_audio_devices() -> Result<Vec<NapiAudioDevice>> {
    let devices = yumo_core::platform::recorder::list_input_devices()
        .map_err(|e| Error::from_reason(format!("Failed to list devices: {e}")))?;

    Ok(devices
        .into_iter()
        .map(|d| NapiAudioDevice {
            id: d.id,
            name: d.name,
            is_default: d.is_default,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Get all settings as a JSON string.
#[napi]
pub fn get_all_settings() -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let settings = db::get_all_settings(&conn)
        .map_err(|e| Error::from_reason(format!("get_all_settings: {e}")))?;
    serde_json::to_string(&settings)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

// ---------------------------------------------------------------------------
// Transcriptions
// ---------------------------------------------------------------------------

/// Get transcriptions with cursor-based pagination.
/// Returns a JSON string of `{ items, next_cursor }`.
#[napi]
pub fn get_transcriptions(
    cursor: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
) -> Result<String> {
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
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// List all available models (local + cloud).
#[napi]
pub fn list_available_models() -> Result<String> {
    let app = ctx()?;
    let models = transcriber::all_models(&app.paths.models_dir);
    serde_json::to_string(&models)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

// ---------------------------------------------------------------------------
// Settings update
// ---------------------------------------------------------------------------

/// Update a single setting.
#[napi]
pub fn update_setting(key: String, value: String) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let json_value: serde_json::Value = serde_json::from_str(&value)
        .unwrap_or_else(|_| serde_json::Value::String(value));
    db::update_setting(&conn, &key, &json_value)
        .map_err(|e| Error::from_reason(format!("update_setting: {e}")))
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Get transcription statistics.
#[napi]
pub fn get_statistics(days: Option<u32>) -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let stats = db::get_statistics(&conn, days.map(|d| d as i64))
        .map_err(|e| Error::from_reason(format!("get_statistics: {e}")))?;
    serde_json::to_string(&stats)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

// ---------------------------------------------------------------------------
// Vocabulary & Replacements
// ---------------------------------------------------------------------------

#[napi]
pub fn get_vocabulary() -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let words = db::get_vocabulary(&conn)
        .map_err(|e| Error::from_reason(format!("get_vocabulary: {e}")))?;
    serde_json::to_string(&words)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

#[napi]
pub fn add_vocabulary(word: String) -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::add_vocabulary(&conn, &word)
        .map_err(|e| Error::from_reason(format!("add_vocabulary: {e}")))
}

#[napi]
pub fn delete_vocabulary(id: String) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::delete_vocabulary(&conn, &id)
        .map_err(|e| Error::from_reason(format!("delete_vocabulary: {e}")))
}

#[napi]
pub fn get_replacements() -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let items = db::get_replacements(&conn)
        .map_err(|e| Error::from_reason(format!("get_replacements: {e}")))?;
    serde_json::to_string(&items)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

#[napi]
pub fn set_replacement(original: String, replacement: String) -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::set_replacement(&conn, &original, &replacement)
        .map_err(|e| Error::from_reason(format!("set_replacement: {e}")))
}

#[napi]
pub fn delete_replacement(id: String) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::delete_replacement(&conn, &id)
        .map_err(|e| Error::from_reason(format!("delete_replacement: {e}")))
}

// ---------------------------------------------------------------------------
// Transcription CRUD
// ---------------------------------------------------------------------------

#[napi]
pub fn delete_transcription(id: String) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::delete_transcription(&conn, &id)
        .map_err(|e| Error::from_reason(format!("delete_transcription: {e}")))
}

#[napi]
pub fn delete_all_transcriptions() -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::delete_all_transcriptions(&conn)
        .map_err(|e| Error::from_reason(format!("delete_all_transcriptions: {e}")))
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

#[napi]
pub fn list_prompts() -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    let prompts = db::list_prompts(&conn)
        .map_err(|e| Error::from_reason(format!("list_prompts: {e}")))?;
    serde_json::to_string(&prompts)
        .map_err(|e| Error::from_reason(format!("JSON serialize: {e}")))
}

#[napi]
pub fn add_prompt(
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<String> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::add_prompt(&conn, &name, &system_msg, &user_msg, false)
        .map_err(|e| Error::from_reason(format!("add_prompt: {e}")))
}

#[napi]
pub fn update_prompt(
    id: String,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::update_prompt(&conn, &id, &name, &system_msg, &user_msg)
        .map_err(|e| Error::from_reason(format!("update_prompt: {e}")))
}

#[napi]
pub fn delete_prompt(id: String) -> Result<()> {
    let app = ctx()?;
    let conn = app.db.lock().map_err(|e| Error::from_reason(format!("DB lock: {e}")))?;
    db::delete_prompt(&conn, &id)
        .map_err(|e| Error::from_reason(format!("delete_prompt: {e}")))
}

// ---------------------------------------------------------------------------
// CSV Import / Export
// ---------------------------------------------------------------------------

#[napi]
pub fn import_dictionary_csv(path: String, dict_type: String) -> Result<()> {
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
}

#[napi]
pub fn export_dictionary_csv(path: String, dict_type: String) -> Result<()> {
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
}

// ---------------------------------------------------------------------------
// Keychain
// ---------------------------------------------------------------------------

#[napi]
pub fn store_api_key(provider: String, key: String) -> Result<()> {
    yumo_core::platform::keychain::store_key("com.voiceink.app", &provider, &key)
        .map_err(|e| Error::from_reason(format!("store_key: {e}")))
}

#[napi]
pub fn get_api_key(provider: String) -> Result<Option<String>> {
    yumo_core::platform::keychain::get_key("com.voiceink.app", &provider)
        .map_err(|e| Error::from_reason(format!("get_key: {e}")))
}

#[napi]
pub fn delete_api_key(provider: String) -> Result<()> {
    yumo_core::platform::keychain::delete_key("com.voiceink.app", &provider)
        .map_err(|e| Error::from_reason(format!("delete_key: {e}")))
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
pub fn daemon_start() -> Result<()> {
    let d = daemon()?;
    d.start().map_err(|e| Error::from_reason(format!("daemon start: {e}")))
}

#[napi]
pub fn daemon_stop() -> Result<()> {
    let d = daemon()?;
    d.stop();
    Ok(())
}

#[napi]
pub fn daemon_load_model(model_repo: String) -> Result<()> {
    let d = daemon()?;
    if !d.is_running() {
        d.start().map_err(|e| Error::from_reason(format!("daemon start: {e}")))?;
    }
    let cmd = serde_json::json!({"action": "load", "model": model_repo});
    let resp = d.send_command(&cmd)
        .map_err(|e| Error::from_reason(format!("daemon load: {e}")))?;
    if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" {
        d.set_loaded_model(Some(model_repo));
        Ok(())
    } else {
        Err(Error::from_reason(resp.error.unwrap_or_else(|| format!("load failed: {}", resp.status))))
    }
}

#[napi]
pub fn daemon_unload_model() -> Result<()> {
    let d = daemon()?;
    let cmd = serde_json::json!({"action": "unload"});
    d.send_command(&cmd)
        .map_err(|e| Error::from_reason(format!("daemon unload: {e}")))?;
    d.set_loaded_model(None);
    Ok(())
}

#[napi]
pub fn daemon_check_deps() -> Result<bool> {
    let d = daemon()?;
    Ok(d.has_python())
}
