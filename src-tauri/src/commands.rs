use std::collections::HashMap;

use log::{info, error};
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
// Frontend log bridge — writes frontend logs to the same log.txt
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "error" => error!("[frontend] {}", message),
        _ => info!("[frontend] {}", message),
    }
}

// ---------------------------------------------------------------------------
// Recording pipeline
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: Option<u32>,
) -> Result<(), AppError> {
    info!("[pipeline] start_recording called, device_id={:?}", device_id);

    // 1. Check we're idle
    {
        let pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        info!("[pipeline] current state: {:?}", *pipeline);
        if *pipeline != PipelineState::Idle {
            error!("[pipeline] not idle, rejecting start_recording");
            return Err(AppError::Recording("Already recording".into()));
        }
    }

    // 2. Get device
    let devices = recorder::list_input_devices();
    info!("[pipeline] found {} input devices", devices.len());
    for d in &devices {
        info!("[pipeline]   device: id={} name={:?} default={}", d.id, d.name, d.is_default);
    }
    if devices.is_empty() {
        error!("[pipeline] no input devices found");
        return Err(AppError::Recording("No input devices found".into()));
    }
    let dev_id = device_id.unwrap_or_else(|| {
        devices
            .iter()
            .find(|d| d.is_default)
            .map(|d| d.id)
            .unwrap_or(devices[0].id)
    });
    info!("[pipeline] using device_id={}", dev_id);

    // 3. Mute system audio if enabled
    let settings = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::get_all_settings(&conn)?
    };
    let mute = settings
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    info!("[pipeline] system_mute_enabled={}", mute);
    if mute {
        audio_ctrl::set_system_muted(true);
    }

    // 4. Start recording
    info!("[pipeline] calling recorder::start_recording...");
    let (handle, _level_rx) = recorder::start_recording(dev_id).map_err(|e| {
        error!("[pipeline] recorder::start_recording failed: {}", e);
        AppError::Recording(e.to_string())
    })?;
    info!("[pipeline] recording started successfully");

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
    info!("[pipeline] state -> Recording");

    // 6. Emit state change
    let _ = app.emit(
        "recording-state",
        serde_json::json!({"state": "recording"}),
    );
    info!("[pipeline] emitted recording-state=recording");

    // 7. Show floating recorder window
    crate::window_manager::WindowManager::new(app.clone()).show("recorder");

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    info!("[pipeline] stop_recording called");

    // 1. Take recording handle
    let handle = {
        state
            .recording_handle
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?
            .take()
            .ok_or_else(|| {
                error!("[pipeline] stop_recording: no recording handle (not recording)");
                AppError::Recording("Not recording".into())
            })?
    };

    // 2. Stop recording
    info!("[pipeline] stopping recorder...");
    let audio_data = recorder::stop_recording(handle).map_err(|e| {
        error!("[pipeline] recorder::stop_recording failed: {}", e);
        AppError::Recording(e.to_string())
    })?;
    let rms = if audio_data.pcm_samples.is_empty() {
        0.0
    } else {
        let sum_sq: f64 = audio_data.pcm_samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum_sq / audio_data.pcm_samples.len() as f64).sqrt()
    };
    info!(
        "[pipeline] recording stopped, samples={} sample_rate={} channels={} rms={:.6}",
        audio_data.pcm_samples.len(),
        audio_data.sample_rate,
        audio_data.channels,
        rms
    );

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
    info!("[pipeline] state -> Transcribing");

    // 4. Read settings
    let settings_map = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        db::get_all_settings(&conn)?
    };
    let model_id = settings_map
        .get("selected_model_id")
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
    info!(
        "[pipeline] settings: model_id={:?} language={:?} enhancement={}",
        model_id, language, enhancement_enabled
    );

    // 5. Transcribe — route by model provider
    let all = transcriber::all_models(&state.paths.models_dir);
    let model_info = all.iter().find(|m| m.id == model_id);
    info!(
        "[pipeline] model lookup: found={} provider={:?}",
        model_info.is_some(),
        model_info.map(|m| format!("{:?}", m.provider))
    );

    info!("[pipeline] effective language: {:?}", language);

    let transcribe_result = match model_info.map(|m| &m.provider) {
        Some(transcriber::ModelProvider::MlxFunASR) => {
            // Auto-start daemon if not running
            if !state.daemon.is_running() {
                info!("[pipeline] daemon not running, starting...");
                if let Err(e) = state.daemon.start() {
                    error!("[pipeline] daemon start failed: {}", e);
                    let mut pipeline = state
                        .pipeline_state
                        .lock()
                        .map_err(|e| AppError::Recording(e.to_string()))?;
                    *pipeline = PipelineState::Idle;
                    let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
                    return Err(e);
                }
                info!("[pipeline] daemon started");
            }
            // Auto-load model if not loaded
            let model_repo = model_info.and_then(|m| m.model_repo.clone());
            let loaded = state.daemon.loaded_model();
            if model_repo.is_some() && loaded.as_ref() != model_repo.as_ref() {
                let repo = model_repo.as_ref().unwrap();
                info!("[pipeline] loading MLX model: {}", repo);
                let cmd = serde_json::json!({"action": "load", "model": repo});
                match state.daemon.send_command(&cmd) {
                    Ok(resp) if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" => {
                        state.daemon.set_loaded_model(model_repo.clone());
                        info!("[pipeline] MLX model loaded");
                    }
                    Ok(resp) => {
                        let msg = resp.error.unwrap_or_else(|| format!("load failed: {}", resp.status));
                        error!("[pipeline] MLX model load failed: {}", msg);
                        let mut pipeline = state
                            .pipeline_state
                            .lock()
                            .map_err(|e| AppError::Recording(e.to_string()))?;
                        *pipeline = PipelineState::Idle;
                        let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
                        return Err(AppError::Transcription(msg));
                    }
                    Err(e) => {
                        error!("[pipeline] MLX model load command failed: {}", e);
                        let mut pipeline = state
                            .pipeline_state
                            .lock()
                            .map_err(|e| AppError::Recording(e.to_string()))?;
                        *pipeline = PipelineState::Idle;
                        let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
                        return Err(e);
                    }
                }
            }
            info!("[pipeline] transcribing via MLX daemon (language={})...", language);
            transcriber::transcribe_via_daemon(
                &state.daemon,
                &audio_data.pcm_samples,
                audio_data.sample_rate,
                &language,
            )
            .await
        }
        _ => {
            let model_path = transcriber::model_path(&state.paths.models_dir, &model_id);
            info!("[pipeline] model path: {:?} exists={}", model_path, model_path.exists());
            if !model_path.exists() {
                error!("[pipeline] model file not found: {:?}", model_path);
                let mut pipeline = state
                    .pipeline_state
                    .lock()
                    .map_err(|e| AppError::Recording(e.to_string()))?;
                *pipeline = PipelineState::Idle;
                let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
                return Err(AppError::Transcription("No model downloaded".into()));
            }
            info!("[pipeline] loading whisper model...");
            let model_path_clone = model_path.clone();
            let samples = audio_data.pcm_samples.clone();
            let sr = audio_data.sample_rate;
            let lang = language.clone();
            tokio::task::spawn_blocking(move || {
                let ctx = transcriber::load_model(&model_path_clone).map_err(|e| {
                    log::error!("[pipeline] load_model failed: {}", e);
                    e
                })?;
                log::info!("[pipeline] transcribing via whisper (language={})...", lang);
                transcriber::transcribe(&ctx, &samples, sr, &lang)
            })
            .await
            .map_err(|e| AppError::Transcription(format!("spawn_blocking: {e}")))?
        }
    };

    // Handle transcription failure — reset pipeline to Idle
    let result = match transcribe_result {
        Ok(r) => r,
        Err(e) => {
            error!("[pipeline] transcription failed: {}", e);
            let mut pipeline = state
                .pipeline_state
                .lock()
                .map_err(|er| AppError::Recording(er.to_string()))?;
            *pipeline = PipelineState::Idle;
            let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
            return Err(e);
        }
    };
    let text = result.text;
    info!(
        "[pipeline] transcription done in {}ms, text length={}, text={:?}",
        result.duration_ms,
        text.len(),
        if text.len() > 200 { &text[..200] } else { &text }
    );

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
    info!("[pipeline] processed text: {:?}", if processed_text.len() > 200 { &processed_text[..200] } else { &processed_text });

    // 7. Optional AI enhancement
    let enhanced_text: Option<String> = if enhancement_enabled {
        info!("[pipeline] enhancement enabled, entering Enhancing state");
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
        info!("[pipeline] enhancement not implemented yet, skipping");
        None
    } else {
        info!("[pipeline] enhancement disabled, skipping");
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

    if permissions::check_accessibility() {
        let restore_delay = settings_map
            .get("clipboard_restore_delay")
            .and_then(|v| v.as_f64())
            .unwrap_or(1500.0) as u64;
        info!("[pipeline] accessibility=true, paste+restore (delay={}ms)", restore_delay);
        paster::paste_text(final_text, restore_delay);
    } else {
        info!("[pipeline] accessibility=false, writing to clipboard only");
        paster::write_clipboard(final_text);
    }

    // 9. Save to DB
    let word_count = final_text.split_whitespace().count() as i32;
    info!("[pipeline] saving to DB, word_count={}", word_count);
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
    info!("[pipeline] saved to DB");

    // 10. Unmute system audio
    if settings_map
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        audio_ctrl::set_system_muted(false);
        info!("[pipeline] unmuted system audio");
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
    info!("[pipeline] stop_recording complete, state -> Idle");

    // Hide floating recorder window
    crate::window_manager::WindowManager::new(app.clone()).hide("recorder");

    Ok(())
}

#[tauri::command]
pub fn cancel_recording(
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), AppError> {
    info!("[cmd] cancel_recording");
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
    crate::window_manager::WindowManager::new(app.clone()).hide("recorder");
    Ok(())
}

#[tauri::command]
pub fn get_pipeline_state(state: State<AppState>) -> Result<serde_json::Value, AppError> {
    let pipeline = state
        .pipeline_state
        .lock()
        .map_err(|e| AppError::Recording(e.to_string()))?;
    let s = match *pipeline {
        PipelineState::Idle => "idle",
        PipelineState::Recording => "recording",
        PipelineState::Transcribing => "transcribing",
        PipelineState::Enhancing => "enhancing",
        PipelineState::Pasting => "pasting",
    };
    Ok(serde_json::json!({"state": s}))
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<recorder::AudioInputDevice> {
    info!("[cmd] list_audio_devices");
    recorder::list_input_devices()
}

#[tauri::command]
pub fn check_permissions() -> permissions::PermissionStatus {
    info!("[cmd] check_permissions");
    permissions::check_all()
}

#[tauri::command]
pub fn request_permission(permission_type: String) {
    info!("[cmd] request_permission: {}", permission_type);
    match permission_type.as_str() {
        "microphone" => permissions::open_microphone_settings(),
        "accessibility" => permissions::open_accessibility_settings(),
        _ => {}
    }
}

#[tauri::command]
pub fn list_available_models(state: State<AppState>) -> Vec<transcriber::ModelInfo> {
    info!("[cmd] list_available_models");
    transcriber::all_models(&state.paths.models_dir)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    info!("[cmd] download_model: {}", model_id);
    let models = transcriber::predefined_models();
    let model = models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| AppError::NotFound(format!("Model {} not found", model_id)))?;

    let dest = transcriber::model_path(&state.paths.models_dir, &model_id);
    std::fs::create_dir_all(&state.paths.models_dir)?;

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
    let path = transcriber::model_path(&state.paths.models_dir, &model_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn import_model(app: AppHandle, state: State<'_, AppState>) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let file_path = app
        .dialog()
        .file()
        .add_filter("Whisper Model", &["bin"])
        .blocking_pick_file();

    let file_path = match file_path {
        Some(f) => f,
        None => return Ok(false),
    };

    let src = file_path
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件路径".into()))?;

    let file_name = src
        .file_name()
        .ok_or_else(|| AppError::Io("无效文件名".into()))?;

    let dest = state.paths.models_dir.join(file_name);
    std::fs::copy(&src, &dest)?;

    Ok(true)
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
    info!("[hotkey] register_hotkey called: {:?}", shortcut);

    // Persist the shortcut string in settings
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_setting(&conn, "hotkey", &Value::String(shortcut.clone()))?;
    drop(conn);

    // Clear any previously registered shortcuts, then register the new one.
    hotkey::unregister_all(&app).map_err(|e| {
        error!("[hotkey] unregister_all failed: {}", e);
        AppError::Io(e.to_string())
    })?;
    let app_clone = app.clone();
    hotkey::register_shortcut(&app, &shortcut, move || {
        info!("[hotkey] shortcut triggered, emitting toggle-recording");
        let _ = app_clone.emit("toggle-recording", ());
    })
    .map_err(|e| {
        error!("[hotkey] register_shortcut failed: {:?} error: {}", shortcut, e);
        AppError::Io(e.to_string())
    })
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

// ---------------------------------------------------------------------------
// MLX FunASR Daemon
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn daemon_start(state: State<AppState>) -> Result<(), AppError> {
    state.daemon.start()
}

#[tauri::command]
pub fn daemon_stop(state: State<AppState>) -> Result<(), AppError> {
    state.daemon.stop();
    Ok(())
}

#[tauri::command]
pub fn daemon_status(state: State<AppState>) -> Result<serde_json::Value, AppError> {
    Ok(serde_json::json!({
        "running": state.daemon.is_running(),
        "loaded_model": state.daemon.loaded_model(),
    }))
}

#[tauri::command]
pub fn daemon_check_deps(state: State<AppState>) -> Result<serde_json::Value, AppError> {
    if !state.daemon.is_running() {
        state.daemon.start()?;
    }
    let cmd = serde_json::json!({"action": "check_dependencies"});
    let resp = state.daemon.send_command(&cmd)?;
    Ok(serde_json::to_value(resp).unwrap_or_default())
}

#[tauri::command]
pub fn daemon_load_model(state: State<AppState>, model_repo: String) -> Result<(), AppError> {
    if !state.daemon.is_running() {
        state.daemon.start()?;
    }
    let cmd = serde_json::json!({"action": "load", "model": model_repo});
    let resp = state.daemon.send_command(&cmd)?;
    if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" {
        state.daemon.set_loaded_model(Some(model_repo));
        Ok(())
    } else {
        Err(AppError::Transcription(
            resp.error.unwrap_or_else(|| format!("Load failed: {}", resp.status))
        ))
    }
}

#[tauri::command]
pub fn daemon_unload_model(state: State<AppState>) -> Result<(), AppError> {
    let cmd = serde_json::json!({"action": "unload"});
    state.daemon.send_command(&cmd)?;
    state.daemon.set_loaded_model(None);
    Ok(())
}

// ---------------------------------------------------------------------------
// Sprite Sheets (shared with native VoiceInk)
// ---------------------------------------------------------------------------

/// List all available sprite sheet manifests.
#[tauri::command]
pub fn list_sprites(state: State<AppState>) -> Result<Vec<Value>, AppError> {
    let base = &state.paths.sprites_dir;
    if !base.exists() {
        return Ok(vec![]);
    }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&base)? {
        let entry = entry?;
        if !entry.path().is_dir() { continue; }
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.exists() {
            let data = std::fs::read_to_string(&manifest_path)?;
            if let Ok(mut manifest) = serde_json::from_str::<Value>(&data) {
                // Add the directory id so frontend knows where to find the image
                let dir_name = entry.file_name().to_string_lossy().to_string();
                manifest.as_object_mut().map(|m| m.insert("dirId".into(), Value::String(dir_name)));
                results.push(manifest);
            }
        }
    }
    Ok(results)
}

/// Read a sprite sheet image file as base64 data URI.
#[tauri::command]
pub fn get_sprite_image(state: State<AppState>, dir_id: String, file_name: String) -> Result<String, AppError> {
    let path = state.paths.sprites_dir.join(&dir_id).join(&file_name);
    if !path.exists() {
        // Try processed version
        let processed = state.paths.sprites_dir.join(&dir_id).join("sprite_processed.png");
        if processed.exists() {
            let data = std::fs::read(&processed)?;
            let b64 = base64_encode(&data);
            return Ok(format!("data:image/png;base64,{}", b64));
        }
        return Err(AppError::NotFound(format!("Sprite image not found: {}", path.display())));
    }
    let data = std::fs::read(&path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime = match ext {
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/png",
    };
    let b64 = base64_encode(&data);
    Ok(format!("data:{};base64,{}", mime, b64))
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn test_base64_encode_one_byte() {
        // "A" -> "QQ=="
        assert_eq!(base64_encode(b"A"), "QQ==");
    }

    #[test]
    fn test_base64_encode_two_bytes() {
        // "AB" -> "QUI="
        assert_eq!(base64_encode(b"AB"), "QUI=");
    }

    #[test]
    fn test_base64_encode_three_bytes() {
        // "ABC" -> "QUJD"
        assert_eq!(base64_encode(b"ABC"), "QUJD");
    }

    #[test]
    fn test_base64_encode_hello_world() {
        assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_base64_encode_binary() {
        assert_eq!(base64_encode(&[0x00, 0xFF, 0x80]), "AP+A");
    }
}
