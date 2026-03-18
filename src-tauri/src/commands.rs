use std::collections::HashMap;

use serde_json::Value;
use tauri::State;

use crate::db::{self, PaginatedResult, Prompt, Replacement, VocabularyWord};
use crate::error::AppError;
use crate::keychain;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Transcriptions
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_transcriptions(
    state: State<AppState>,
    cursor: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<PaginatedResult, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_transcriptions(&conn, cursor.as_deref(), query.as_deref(), limit.unwrap_or(20))
}

#[tauri::command]
pub fn delete_transcription(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_transcription(&conn, &id)
}

#[tauri::command]
pub fn delete_all_transcriptions(state: State<AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_all_transcriptions(&conn)
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_vocabulary(state: State<AppState>) -> Result<Vec<VocabularyWord>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_vocabulary(&conn)
}

#[tauri::command]
pub fn add_vocabulary(state: State<AppState>, word: String) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::add_vocabulary(&conn, &word)
}

#[tauri::command]
pub fn delete_vocabulary(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_vocabulary(&conn, &id)
}

// ---------------------------------------------------------------------------
// Replacements
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_replacements(state: State<AppState>) -> Result<Vec<Replacement>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_replacements(&conn)
}

#[tauri::command]
pub fn set_replacement(
    state: State<AppState>,
    original: String,
    replacement: String,
) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::set_replacement(&conn, &original, &replacement)
}

#[tauri::command]
pub fn delete_replacement(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_replacement(&conn, &id)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<HashMap<String, Value>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_all_settings(&conn)
}

#[tauri::command]
pub fn update_setting(
    state: State<AppState>,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_setting(&conn, &key, &value)
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_prompts(state: State<AppState>) -> Result<Vec<Prompt>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::list_prompts(&conn)
}

#[tauri::command]
pub fn add_prompt(
    state: State<AppState>,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::add_prompt(&conn, &name, &system_msg, &user_msg, false)
}

#[tauri::command]
pub fn update_prompt(
    state: State<AppState>,
    id: String,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_prompt(&conn, &id, &name, &system_msg, &user_msg)
}

#[tauri::command]
pub fn delete_prompt(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_prompt(&conn, &id)
}

// ---------------------------------------------------------------------------
// Convenience: select prompt / model (stored in settings)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn select_prompt(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_setting(&conn, "selected_prompt_id", &Value::String(id))
}

#[tauri::command]
pub fn select_model(state: State<AppState>, model_id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_setting(&conn, "selected_model_id", &Value::String(model_id))
}

// ---------------------------------------------------------------------------
// Keychain (API key storage)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn store_api_key(provider: String, key: String) -> Result<(), AppError> {
    keychain::store_key("com.voiceink.app", &provider, &key)
}

#[tauri::command]
pub fn get_api_key(provider: String) -> Result<Option<String>, AppError> {
    keychain::get_key("com.voiceink.app", &provider)
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), AppError> {
    keychain::delete_key("com.voiceink.app", &provider)
}
