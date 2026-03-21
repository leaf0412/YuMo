use std::sync::OnceLock;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use yumo_core::db;
use yumo_core::state::{AppContext, AppPaths};

// ---------------------------------------------------------------------------
// Global AppContext (initialized once via `init`)
// ---------------------------------------------------------------------------

static APP_CTX: OnceLock<AppContext> = OnceLock::new();

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
    paths.denoiser_dir = std::path::PathBuf::from(&data_dir).join("denoiser");

    // Ensure data directory exists
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| Error::from_reason(format!("Failed to create data dir: {e}")))?;

    let db_path = std::path::PathBuf::from(&data_dir).join("yumo.db");
    let conn = db::init_database(&db_path)
        .map_err(|e| Error::from_reason(format!("Failed to init database: {e}")))?;

    let app_ctx = AppContext::new(conn, paths);

    APP_CTX
        .set(app_ctx)
        .map_err(|_| Error::from_reason("AppContext already initialized"))?;

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
