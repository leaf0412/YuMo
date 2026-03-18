use std::collections::HashMap;

use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::db::{self, PaginatedResult, Prompt, Replacement, VocabularyWord};
use crate::error::AppError;
use crate::hotkey;
use crate::keychain;
use crate::pipeline::PipelineState;
use crate::state::AppState;
use crate::{audio_ctrl, paster, permissions, recorder, text_processor, transcriber};

// ---------------------------------------------------------------------------
// Recording pipeline
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: Option<u32>,
) -> Result<(), AppError> {
    // 1. Check we're idle
    {
        let pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        if *pipeline != PipelineState::Idle {
            return Err(AppError::Recording("Already recording".into()));
        }
    }

    // 2. Get device
    let devices = recorder::list_input_devices();
    if devices.is_empty() {
        return Err(AppError::Recording("No input devices found".into()));
    }
    let dev_id = device_id.unwrap_or_else(|| {
        devices
            .iter()
            .find(|d| d.is_default)
            .map(|d| d.id)
            .unwrap_or(devices[0].id)
    });

    // 3. Mute system audio if enabled
    let settings = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::get_all_settings(&conn)?
    };
    if settings
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        audio_ctrl::set_system_muted(true);
    }

    // 4. Start recording
    let (handle, _level_rx) =
        recorder::start_recording(dev_id).map_err(|e| AppError::Recording(e.to_string()))?;

    // 5. Store handle and update state
    {
        let mut rec = state
            .recording_handle
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *rec = Some(handle);
    }
    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Recording;
    }

    // 6. Emit state change
    let _ = app.emit(
        "recording-state",
        serde_json::json!({"state": "recording"}),
    );

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // 1. Take recording handle
    let handle = {
        state
            .recording_handle
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .take()
            .ok_or_else(|| AppError::Recording("Not recording".into()))?
    };

    // 2. Stop recording
    let audio_data =
        recorder::stop_recording(handle).map_err(|e| AppError::Recording(e.to_string()))?;

    // 3. Update state -> Transcribing
    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Transcribing;
    }
    let _ = app.emit(
        "recording-state",
        serde_json::json!({"state": "transcribing"}),
    );

    // 4. Read settings
    let settings_map = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::get_all_settings(&conn)?
    };
    let model_id = settings_map
        .get("selected_model")
        .and_then(|v| v.as_str())
        .unwrap_or("ggml-base.en")
        .to_string();
    let language = settings_map
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_string();
    let enhancement_enabled = settings_map
        .get("enhancement_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 5. Transcribe
    let model_path = transcriber::model_path(&state.models_dir, &model_id);
    if !model_path.exists() {
        // Reset state back to idle on error
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Idle;
        let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
        return Err(AppError::Transcription("No model downloaded".into()));
    }
    let ctx = transcriber::load_model(&model_path)?;
    let result = transcriber::transcribe(
        &ctx,
        &audio_data.pcm_samples,
        audio_data.sample_rate,
        &language,
    )?;
    let text = result.text;

    // 6. Apply text processing
    let replacements: Vec<(String, String)> = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::get_replacements(&conn)?
            .into_iter()
            .map(|r| (r.original, r.replacement))
            .collect()
    };
    let auto_capitalize = settings_map
        .get("auto_capitalize")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let processed_text = text_processor::process_text(&text, &replacements, auto_capitalize);

    // 7. Optional AI enhancement
    let enhanced_text: Option<String> = if enhancement_enabled {
        {
            let mut pipeline = state
                .pipeline_state
                .lock()
                .map_err(|e| AppError::Recording(e.to_string()))?;
            *pipeline = PipelineState::Enhancing;
        }
        let _ = app.emit(
            "recording-state",
            serde_json::json!({"state": "enhancing"}),
        );

        // TODO: integrate with keychain and enhancer module
        // For now, skip enhancement if no API key is configured
        None
    } else {
        None
    };

    // 8. Paste
    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Pasting;
    }
    let _ = app.emit(
        "recording-state",
        serde_json::json!({"state": "pasting"}),
    );

    let final_text = enhanced_text.as_deref().unwrap_or(&processed_text);
    let restore_delay = settings_map
        .get("clipboard_restore_delay")
        .and_then(|v| v.as_f64())
        .unwrap_or(1500.0) as u64;
    paster::paste_text(final_text, restore_delay);

    // 9. Save to DB
    let word_count = final_text.split_whitespace().count() as i32;
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::insert_transcription(
            &conn,
            &processed_text,
            enhanced_text.as_deref(),
            0.0, // TODO: actual audio duration
            &model_id,
            word_count,
        )?;
    }

    // 10. Unmute system audio
    if settings_map
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        audio_ctrl::set_system_muted(false);
    }

    // 11. Back to idle
    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Idle;
    }
    let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
    let _ = app.emit(
        "transcription-result",
        serde_json::json!({
            "text": processed_text,
            "enhanced_text": enhanced_text,
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn cancel_recording(
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), AppError> {
    let handle = state
        .recording_handle
        .lock()
        .map_err(|e| AppError::Recording(e.to_string()))?
        .take();
    if let Some(h) = handle {
        let _ = recorder::cancel_recording(h);
    }

    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Idle;
    }

    // Unmute if needed
    if let Ok(conn) = state.db.lock() {
        if let Ok(settings) = db::get_all_settings(&conn) {
            if settings
                .get("system_mute_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                audio_ctrl::set_system_muted(false);
            }
        }
    }

    let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
    Ok(())
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<recorder::AudioInputDevice> {
    recorder::list_input_devices()
}

#[tauri::command]
pub fn check_permissions() -> permissions::PermissionStatus {
    permissions::check_all()
}

#[tauri::command]
pub fn request_permission(permission_type: String) {
    match permission_type.as_str() {
        "microphone" => permissions::open_microphone_settings(),
        "accessibility" => permissions::open_accessibility_settings(),
        _ => {}
    }
}

#[tauri::command]
pub fn list_available_models(state: State<AppState>) -> Vec<transcriber::ModelInfo> {
    transcriber::check_downloaded_models(&state.models_dir)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    let models = transcriber::predefined_models();
    let model = models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| AppError::NotFound(format!("Model {} not found", model_id)))?;

    let dest = transcriber::model_path(&state.models_dir, &model_id);
    std::fs::create_dir_all(&state.models_dir)?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let url = model.download_url.clone();
    let app_clone = app.clone();
    let model_id_clone = model_id.clone();

    // Spawn progress listener
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = app_clone.emit(
                "model-download-progress",
                serde_json::json!({
                    "model_id": model_id_clone,
                    "progress": progress,
                }),
            );
        }
    });

    // Download
    crate::downloader::download_file(&url, &dest, Some(tx)).await?;

    Ok(())
}

#[tauri::command]
pub fn delete_model(state: State<AppState>, model_id: String) -> Result<(), AppError> {
    let path = transcriber::model_path(&state.models_dir, &model_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

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

// ---------------------------------------------------------------------------
// Hotkey
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn register_hotkey(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<(), AppError> {
    // Persist the shortcut string in settings
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_setting(&conn, "hotkey", &Value::String(shortcut.clone()))?;
    drop(conn);

    // Clear any previously registered shortcuts, then register the new one.
    hotkey::unregister_all(&app).map_err(|e| AppError::Io(e.to_string()))?;
    hotkey::register_shortcut(&app, &shortcut, || {
        // TODO: trigger recording toggle via app event
    })
    .map_err(|e| AppError::Io(e.to_string()))
}

#[tauri::command]
pub fn unregister_hotkey(app: AppHandle) -> Result<(), AppError> {
    hotkey::unregister_all(&app).map_err(|e| AppError::Io(e.to_string()))
}

// ---------------------------------------------------------------------------
// Dictionary CSV Import / Export
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn import_dictionary_csv(state: State<AppState>, path: String, dict_type: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    let path = std::path::Path::new(&path);
    match dict_type.as_str() {
        "vocabulary" => db::import_vocabulary_csv(&conn, path),
        "replacements" => db::import_replacements_csv(&conn, path),
        _ => Err(AppError::InvalidInput("Unknown dict type".into())),
    }
}

#[tauri::command]
pub fn export_dictionary_csv(state: State<AppState>, path: String, dict_type: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    let path = std::path::Path::new(&path);
    match dict_type.as_str() {
        "vocabulary" => db::export_vocabulary_csv(&conn, path),
        "replacements" => db::export_replacements_csv(&conn, path),
        _ => Err(AppError::InvalidInput("Unknown dict type".into())),
    }
}
