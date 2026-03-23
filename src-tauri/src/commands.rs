use std::collections::HashMap;

use log::{info, error};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::db::{self, PaginatedResult, Prompt, Replacement, VocabularyWord};
use crate::error::AppError;
use crate::mask;
use crate::hotkey;
use crate::pipeline::PipelineState;
use crate::state::AppContext;
use crate::daemon::DaemonManager;
use crate::{audio_io, platform, text_processor, transcriber};
use crate::platform::{audio_ctrl, keychain, paster, permissions};

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
    state: State<'_, AppContext>,
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

    // 2. Read settings from cache + mute BEFORE recording to avoid capturing system sound
    let settings = state.settings_cache.read()
        .map_err(|e| AppError::Recording(e.to_string()))?
        .clone();
    let mute = settings
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if mute {
        info!("[pipeline] muting system audio");
        let _ = audio_ctrl::set_system_muted(true);
    }

    // 3. Resolve target device from cache (fallback to fresh enumeration)
    let devices = {
        let cached = state.device_cache.read()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        if cached.is_empty() {
            drop(cached);
            let fresh = platform::recorder::list_input_devices()?;
            info!("[pipeline] cache empty, enumerated {} devices", fresh.len());
            let mut cache = state.device_cache.write()
                .map_err(|e| AppError::Recording(e.to_string()))?;
            *cache = fresh.clone();
            fresh
        } else {
            cached.clone()
        }
    };
    info!("[pipeline] found {} input devices", devices.len());
    if devices.is_empty() {
        error!("[pipeline] no input devices found");
        return Err(AppError::Recording("No input devices found".into()));
    }
    let dev_id = device_id.unwrap_or_else(|| {
        // Prefer saved device if still available
        let saved = settings.get("audio_device")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        if let Some(id) = saved {
            if devices.iter().any(|d| d.id == id) {
                return id;
            }
            info!("[pipeline] saved device_id={} not found, using default", id);
        }
        devices.iter().find(|d| d.is_default).map(|d| d.id).unwrap_or(devices[0].id)
    });
    info!("[pipeline] using device_id={}", dev_id);

    info!("[pipeline] calling recorder::start_recording...");
    let (handle, _level_rx) = platform::recorder::start_recording(dev_id).map_err(|e| {
        error!("[pipeline] recorder::start_recording failed: {}", e);
        AppError::Recording(e.to_string())
    })?;
    info!("[pipeline] recording started successfully");

    // 4. Store handle and update state
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

    // 7. Register Escape as global shortcut for cancel during recording
    {
        let app_esc = app.clone();
        let _ = hotkey::register_escape(&app, move || {
            use tauri::Emitter;
            info!("[hotkey] Escape pressed during recording");
            let _ = app_esc.emit("escape-pressed", ());
        });
    }

    // 8. Show floating recorder window
    crate::window_manager::WindowManager::new(app.clone()).show("recorder");

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppContext>,
    daemon: State<'_, DaemonManager>,
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

    // 1.5 Immediately update state so frontend knows we're processing
    //     (prevents "not recording" errors on repeated hotkey presses)
    {
        let mut pipeline = state
            .pipeline_state
            .lock()
            .map_err(|e| AppError::Recording(e.to_string()))?;
        *pipeline = PipelineState::Processing;
    }
    let _ = app.emit(
        "recording-state",
        serde_json::json!({"state": "processing"}),
    );
    info!("[pipeline] state -> Processing (handle taken, stopping recorder)");

    let pipeline_start = std::time::Instant::now();

    // 2. Stop recording
    info!("[pipeline] stopping recorder...");
    let audio_data = platform::recorder::stop_recording(handle).map_err(|e| {
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
    let audio_duration_secs = if audio_data.sample_rate > 0 && audio_data.channels > 0 {
        audio_data.pcm_samples.len() as f64 / audio_data.sample_rate as f64 / audio_data.channels as f64
    } else {
        0.0
    };
    info!("[pipeline] audio duration: {:.2}s", audio_duration_secs);

    // 2.5 Save recording WAV file
    let recording_path = match audio_io::save_recording(&audio_data, &state.paths.recordings_dir) {
        Ok(path) => {
            info!("[pipeline] recording saved: {}", path.display());
            Some(path.to_string_lossy().to_string())
        }
        Err(e) => {
            error!("[pipeline] failed to save recording: {}", e);
            None
        }
    };

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
    let temperature_key = format!("model_{}_temperature", model_id);
    let max_tokens_key = format!("model_{}_max_tokens", model_id);
    let temperature: f64 = settings_map
        .get(&temperature_key)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let max_tokens: u32 = settings_map
        .get(&max_tokens_key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(1900);
    info!(
        "[pipeline] settings: model_id={:?} language={:?} enhancement={} temperature={} max_tokens={}",
        model_id, language, enhancement_enabled, temperature, max_tokens
    );

    // 5. Transcribe — route by model provider
    let transcribe_start = std::time::Instant::now();
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
            if !daemon.is_running() {
                info!("[pipeline] daemon not running, starting...");
                if let Err(e) = daemon.start() {
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
            let loaded = daemon.loaded_model();
            if model_repo.is_some() && loaded.as_ref() != model_repo.as_ref() {
                let repo = model_repo.as_ref().unwrap();
                info!("[pipeline] loading MLX model: {}", repo);
                let cmd = serde_json::json!({"action": "load", "model": repo});
                match daemon.send_command(&cmd) {
                    Ok(resp) if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" => {
                        daemon.set_loaded_model(model_repo.clone());
                        let _ = app.emit("daemon-status-changed", ());
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
                &*daemon,
                &audio_data.pcm_samples,
                audio_data.sample_rate,
                &language,
                temperature,
                max_tokens,
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
            let temp = temperature as f32;
            tokio::task::spawn_blocking(move || {
                let ctx = transcriber::load_model(&model_path_clone).map_err(|e| {
                    log::error!("[pipeline] load_model failed: {}", e);
                    e
                })?;
                log::info!("[pipeline] transcribing via whisper (language={})...", lang);
                transcriber::transcribe(&ctx, &samples, sr, &lang, temp)
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
        "[pipeline] transcription done in {}ms, text={}",
        result.duration_ms,
        mask::mask_text(&text)
    );
    info!("[pipeline] [transcribe_complete] elapsed_ms={} text_len={}", transcribe_start.elapsed().as_millis(), text.len());

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
    info!("[pipeline] processed text={}", mask::mask_text(&processed_text));

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
        let enhance_start = std::time::Instant::now();
        info!("[pipeline] enhancement not implemented yet, skipping");
        let enhanced_text_inner: Option<String> = None;
        if let Some(ref et) = enhanced_text_inner {
            info!("[pipeline] [enhance_complete] elapsed_ms={} text_len={}", enhance_start.elapsed().as_millis(), et.len());
        }
        enhanced_text_inner
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

    let paste_error: Option<String> = if permissions::check_accessibility() {
        let restore_delay = settings_map
            .get("clipboard_restore_delay")
            .and_then(|v| v.as_f64())
            .unwrap_or(1500.0) as u64;
        info!("[pipeline] accessibility=true, paste+restore (delay={}ms)", restore_delay);
        #[cfg(target_os = "linux")]
        {
            paster::paste_text(final_text, restore_delay)
                .err()
                .map(|e| {
                    error!("[pipeline] paste_text failed: {}", e);
                    e.to_string()
                })
        }
        #[cfg(not(target_os = "linux"))]
        {
            paster::paste_text(final_text, restore_delay);
            None
        }
    } else {
        info!("[pipeline] accessibility=false, writing to clipboard only");
        paster::write_clipboard(final_text);
        None
    };

    // 9. Save to DB
    let word_count = db::count_words(final_text) as i32;
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
            audio_duration_secs,
            &model_id,
            word_count,
            recording_path.as_deref(),
        )?;
    }
    info!("[pipeline] saved to DB");

    // 9.1 Save transcription text alongside the WAV file
    if let Some(ref wav_path) = recording_path {
        let txt_path = std::path::Path::new(wav_path).with_extension("txt");
        match std::fs::write(&txt_path, final_text) {
            Ok(_) => info!("[pipeline] transcription saved: {}", txt_path.display()),
            Err(e) => error!("[pipeline] failed to save transcription txt: {}", e),
        }
    }

    // 10. Unmute system audio
    if settings_map
        .get("system_mute_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let _ = audio_ctrl::set_system_muted(false);
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
    if let Some(ref err) = paste_error {
        let _ = app.emit("paste-failed", serde_json::json!({"error": err}));
    }
    info!("[pipeline] stop_recording complete, state -> Idle");

    // Unregister Escape shortcut
    let _ = hotkey::unregister_escape(&app);

    // Hide floating recorder window
    crate::window_manager::WindowManager::new(app.clone()).hide("recorder");

    info!("[pipeline] [complete] total_ms={}", pipeline_start.elapsed().as_millis());
    Ok(())
}

#[tauri::command]
pub fn cancel_recording(
    app: AppHandle,
    state: State<AppContext>,
) -> Result<(), AppError> {
    info!("[cmd] cancel_recording");
    let handle = state
        .recording_handle
        .lock()
        .map_err(|e| AppError::Recording(e.to_string()))?
        .take();
    if let Some(h) = handle {
        let _ = platform::recorder::cancel_recording(h);
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
                let _ = audio_ctrl::set_system_muted(false);
            }
        }
    }

    // Unregister Escape shortcut
    let _ = hotkey::unregister_escape(&app);

    let _ = app.emit("recording-state", serde_json::json!({"state": "idle"}));
    crate::window_manager::WindowManager::new(app.clone()).hide("recorder");
    Ok(())
}

#[tauri::command]
pub fn get_pipeline_state(state: State<AppContext>) -> Result<serde_json::Value, AppError> {
    let pipeline = state
        .pipeline_state
        .lock()
        .map_err(|e| AppError::Recording(e.to_string()))?;
    let s = match *pipeline {
        PipelineState::Idle => "idle",
        PipelineState::Recording => "recording",
        PipelineState::Processing => "processing",
        PipelineState::Transcribing => "transcribing",
        PipelineState::Enhancing => "enhancing",
        PipelineState::Pasting => "pasting",
    };
    Ok(serde_json::json!({"state": s}))
}

#[tauri::command]
pub fn get_statistics(
    state: State<AppContext>,
    days: Option<i64>,
) -> Result<db::Statistics, AppError> {
    info!("[cmd] get_statistics days={:?}", days);
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Database(e.to_string()))?;
    db::get_statistics(&conn, days)
}

#[tauri::command]
pub fn import_voiceink_legacy(
    state: State<AppContext>,
    store_path: String,
) -> Result<db::ImportResult, AppError> {
    info!("[cmd] import_voiceink_legacy store_path={}", store_path);

    let store = std::path::Path::new(&store_path);
    if !store.exists() {
        return Err(AppError::InvalidInput("VoiceInk database file not found".into()));
    }

    // Dictionary store is alongside the main store
    let dict_store = store.parent()
        .map(|p| p.join("dictionary.store"));
    let dict_ref = dict_store.as_deref()
        .filter(|p| p.exists());

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Database(e.to_string()))?;

    db::import_voiceink_legacy(&conn, store, dict_ref, &state.paths.recordings_dir)
}

/// Auto-detect VoiceInk macOS data directory
#[tauri::command]
pub fn detect_voiceink_legacy_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let store = home
        .join("Library/Application Support/com.prakashjoshipax.VoiceInk/default.store");
    if store.exists() {
        Some(store.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Let user pick a .store file via file dialog, then import it.
#[tauri::command]
pub async fn import_voiceink_from_dialog(
    app: AppHandle,
    state: State<'_, AppContext>,
) -> Result<db::ImportResult, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("VoiceInk Database", &["store"])
        .set_title("选择 VoiceInk 数据库文件")
        .blocking_pick_file();

    let file = match file {
        Some(f) => f,
        None => return Err(AppError::InvalidInput("用户取消选择".into())),
    };

    let store = file
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件路径".into()))?;

    if !store.exists() {
        return Err(AppError::InvalidInput("所选文件不存在".into()));
    }

    // Validate file extension
    let ext = store.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "store" {
        return Err(AppError::InvalidInput(
            "请选择 .store 格式的 VoiceInk 数据库文件".into(),
        ));
    }

    info!("[cmd] import_voiceink_from_dialog store={}", store.display());

    let dict_store = store.parent().map(|p| p.join("dictionary.store"));
    let dict_ref = dict_store.as_deref().filter(|p| p.exists());

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Database(e.to_string()))?;

    db::import_voiceink_legacy(&conn, store, dict_ref, &state.paths.recordings_dir)
}

#[tauri::command]
pub fn list_audio_devices(
    state: State<AppContext>,
) -> Result<Vec<platform::AudioInputDevice>, AppError> {
    info!("[cmd] list_audio_devices");
    let devices = platform::recorder::list_input_devices()?;
    // Refresh device cache
    if let Ok(mut cache) = state.device_cache.write() {
        *cache = devices.clone();
    }
    Ok(devices)
}

#[tauri::command]
pub fn check_permissions() -> platform::PermissionStatus {
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
pub fn list_available_models(state: State<AppContext>) -> Vec<transcriber::ModelInfo> {
    info!("[cmd] list_available_models");
    transcriber::all_models(&state.paths.models_dir)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppContext>,
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
pub fn delete_model(state: State<AppContext>, daemon: State<DaemonManager>, model_id: String) -> Result<(), AppError> {
    info!("[cmd] delete_model id={}", model_id);
    let all = transcriber::all_models(&state.paths.models_dir);
    let model_info = all.iter().find(|m| m.id == model_id);

    if let Some(model) = model_info {
        match model.provider {
            transcriber::ModelProvider::MlxWhisper | transcriber::ModelProvider::MlxFunASR => {
                // Delete HuggingFace cache directory for MLX models
                if let Some(repo) = &model.model_repo {
                    let cache_name = repo.replace('/', "--");
                    let cache_dir = state.paths.models_dir.join(format!("models--{}", cache_name));
                    if cache_dir.exists() {
                        info!("[cmd] deleting MLX cache: {:?}", cache_dir);
                        std::fs::remove_dir_all(&cache_dir)?;
                    }
                }
                // If this model is currently loaded, unload it
                if daemon.loaded_model().as_deref() == model.model_repo.as_deref() {
                    let cmd = serde_json::json!({"action": "unload"});
                    let _ = daemon.send_command(&cmd);
                    daemon.set_loaded_model(None);
                }
            }
            _ => {
                // Delete local whisper .bin file
                let path = transcriber::model_path(&state.paths.models_dir, &model_id);
                if path.exists() {
                    info!("[cmd] deleting model file: {:?}", path);
                    std::fs::remove_file(&path)?;
                }
            }
        }
    }

    // Clear selected_model_id if this was the selected model
    {
        let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
        let selected = db::get_setting(&conn, "selected_model_id")?;
        if selected.as_ref().and_then(|v| v.as_str()) == Some(&model_id) {
            drop(conn);
            state.set_setting_cached("selected_model_id", &serde_json::json!(""))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn import_model(app: AppHandle, state: State<'_, AppContext>) -> Result<bool, AppError> {
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
    state: State<AppContext>,
    cursor: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<PaginatedResult, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_transcriptions(&conn, cursor.as_deref(), query.as_deref(), limit.unwrap_or(20))
}

#[tauri::command]
pub fn get_recording(recording_path: String) -> Result<String, AppError> {
    let path = std::path::Path::new(&recording_path);
    audio_io::read_recording_as_data_uri(path)
}

#[tauri::command]
pub fn delete_transcription(state: State<AppContext>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_transcription(&conn, &id)
}

#[tauri::command]
pub fn delete_all_transcriptions(state: State<AppContext>) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_all_transcriptions(&conn)
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_vocabulary(state: State<AppContext>) -> Result<Vec<VocabularyWord>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_vocabulary(&conn)
}

#[tauri::command]
pub fn add_vocabulary(state: State<AppContext>, word: String) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::add_vocabulary(&conn, &word)
}

#[tauri::command]
pub fn delete_vocabulary(state: State<AppContext>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_vocabulary(&conn, &id)
}

// ---------------------------------------------------------------------------
// Replacements
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_replacements(state: State<AppContext>) -> Result<Vec<Replacement>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_replacements(&conn)
}

#[tauri::command]
pub fn set_replacement(
    state: State<AppContext>,
    original: String,
    replacement: String,
) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::set_replacement(&conn, &original, &replacement)
}

#[tauri::command]
pub fn delete_replacement(state: State<AppContext>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_replacement(&conn, &id)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<AppContext>) -> Result<HashMap<String, Value>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::get_all_settings(&conn)
}

#[tauri::command]
pub fn update_setting(
    state: State<AppContext>,
    key: String,
    value: Value,
) -> Result<(), AppError> {
    state.set_setting_cached(&key, &value)
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_prompts(state: State<AppContext>) -> Result<Vec<Prompt>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::list_prompts(&conn)
}

#[tauri::command]
pub fn add_prompt(
    state: State<AppContext>,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<String, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::add_prompt(&conn, &name, &system_msg, &user_msg, false)
}

#[tauri::command]
pub fn update_prompt(
    state: State<AppContext>,
    id: String,
    name: String,
    system_msg: String,
    user_msg: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::update_prompt(&conn, &id, &name, &system_msg, &user_msg)
}

#[tauri::command]
pub fn delete_prompt(state: State<AppContext>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    db::delete_prompt(&conn, &id)
}

// ---------------------------------------------------------------------------
// Convenience: select prompt / model (stored in settings)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn select_prompt(state: State<AppContext>, id: String) -> Result<(), AppError> {
    state.set_setting_cached("selected_prompt_id", &Value::String(id))
}

#[tauri::command]
pub fn select_model(state: State<AppContext>, model_id: String) -> Result<(), AppError> {
    state.set_setting_cached("selected_model_id", &Value::String(model_id))
}

// ---------------------------------------------------------------------------
// Keychain (API key storage)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn store_api_key(provider: String, key: String) -> Result<(), AppError> {
    info!("[cmd] store_api_key provider={} key={}", provider, mask::mask(&key));
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
    state: State<AppContext>,
    shortcut: String,
) -> Result<(), AppError> {
    info!("[hotkey] register_hotkey called: {:?}", shortcut);

    // Persist the shortcut string in settings
    state.set_setting_cached("hotkey", &Value::String(shortcut.clone()))?;

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
pub fn import_dictionary_csv(state: State<AppContext>, path: String, dict_type: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    let path = std::path::Path::new(&path);
    match dict_type.as_str() {
        "vocabulary" => db::import_vocabulary_csv(&conn, path),
        "replacements" => db::import_replacements_csv(&conn, path),
        _ => Err(AppError::InvalidInput("Unknown dict type".into())),
    }
}

#[tauri::command]
pub fn export_dictionary_csv(state: State<AppContext>, path: String, dict_type: String) -> Result<(), AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    let path = std::path::Path::new(&path);
    match dict_type.as_str() {
        "vocabulary" => db::export_vocabulary_csv(&conn, path),
        "replacements" => db::export_replacements_csv(&conn, path),
        _ => Err(AppError::InvalidInput("Unknown dict type".into())),
    }
}

/// Let user pick a .csv file via file dialog, then import it into the dictionary.
#[tauri::command]
pub async fn import_dictionary_csv_dialog(
    app: AppHandle,
    state: State<'_, AppContext>,
    dict_type: String,
) -> Result<(), AppError> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("CSV", &["csv"])
        .set_title("选择 CSV 文件")
        .blocking_pick_file();

    let file = match file {
        Some(f) => f,
        None => return Err(AppError::Cancelled),
    };

    let path = file
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件路径".into()))?;

    info!("[cmd] import_dictionary_csv_dialog path={}, type={}", path.display(), dict_type);

    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
    match dict_type.as_str() {
        "vocabulary" => db::import_vocabulary_csv(&conn, path),
        "replacements" => db::import_replacements_csv(&conn, path),
        _ => Err(AppError::InvalidInput("Unknown dict type".into())),
    }
}

/// Let user choose a save path via file dialog, then export dictionary to CSV.
#[tauri::command]
pub async fn export_dictionary_csv_dialog(
    app: AppHandle,
    state: State<'_, AppContext>,
    dict_type: String,
) -> Result<(), AppError> {
    use tauri_plugin_dialog::DialogExt;

    let default_name = match dict_type.as_str() {
        "vocabulary" => "vocabulary.csv",
        "replacements" => "replacements.csv",
        _ => "dictionary.csv",
    };

    let file = app
        .dialog()
        .file()
        .add_filter("CSV", &["csv"])
        .set_file_name(default_name)
        .set_title("导出 CSV 文件")
        .blocking_save_file();

    let file = match file {
        Some(f) => f,
        None => return Err(AppError::Cancelled),
    };

    let path = file
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件路径".into()))?;

    info!("[cmd] export_dictionary_csv_dialog path={}, type={}", path.display(), dict_type);

    let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
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
pub fn daemon_start(app: tauri::AppHandle, daemon: State<DaemonManager>) -> Result<(), AppError> {
    let result = daemon.start();
    if result.is_ok() {
        let _ = app.emit("daemon-status-changed", ());
    }
    result
}

#[tauri::command]
pub fn daemon_stop(app: tauri::AppHandle, daemon: State<DaemonManager>) -> Result<(), AppError> {
    daemon.stop();
    let _ = app.emit("daemon-status-changed", ());
    Ok(())
}

#[tauri::command]
pub fn daemon_status(daemon: State<DaemonManager>) -> Result<serde_json::Value, AppError> {
    use std::sync::LazyLock;
    use std::sync::Mutex;
    use std::time::Instant;

    static LAST_LOG: LazyLock<Mutex<(Instant, bool, Option<String>)>> =
        LazyLock::new(|| Mutex::new((Instant::now(), false, None)));

    let running = daemon.is_running();
    let loaded = daemon.loaded_model();

    // Only log on state change or every 30 seconds
    if let Ok(mut last) = LAST_LOG.lock() {
        let changed = last.1 != running || last.2 != loaded;
        let elapsed = last.0.elapsed().as_secs() >= 30;
        if changed || elapsed {
            info!("[daemon] [status] running={} loaded_model={:?}", running, loaded);
            *last = (Instant::now(), running, loaded.clone());
        }
    }

    Ok(serde_json::json!({
        "running": running,
        "loaded_model": loaded,
    }))
}

#[tauri::command]
pub fn daemon_check_deps(daemon: State<DaemonManager>) -> Result<serde_json::Value, AppError> {
    if !daemon.is_running() {
        daemon.start()?;
    }
    let cmd = serde_json::json!({"action": "check_dependencies"});
    let resp = daemon.send_command(&cmd)?;
    Ok(serde_json::to_value(resp).unwrap_or_default())
}

#[tauri::command]
pub async fn daemon_load_model(
    app: tauri::AppHandle,
    daemon: State<'_, DaemonManager>,
    model_repo: String,
) -> Result<(), AppError> {
    use crate::daemon::DaemonEventCallback;

    if !daemon.is_running() {
        // Emit setup stages so frontend can show progress
        let _ = app.emit("daemon-setup-status", serde_json::json!({"stage": "checking_python"}));

        if !daemon.has_python() {
            // Build a callback that bridges to tauri::Emitter
            let app_for_setup = app.clone();
            let setup_cb: DaemonEventCallback = Box::new(move |event_name, payload| {
                use tauri::Emitter;
                let _ = app_for_setup.emit(event_name, payload.clone());
            });
            tokio::task::spawn_blocking(move || DaemonManager::ensure_python_static(Some(&setup_cb)))
                .await
                .map_err(|e| AppError::Transcription(format!("setup failed: {e}")))?
                .map_err(|e| {
                    log::error!("[cmd] python setup failed: {}", e);
                    e
                })?;
        }

        let _ = app.emit("daemon-setup-status", serde_json::json!({"stage": "starting_daemon"}));
        daemon.start()?;
        let _ = app.emit("daemon-setup-status", serde_json::json!({"stage": "ready"}));
    }
    let cmd = serde_json::json!({"action": "load", "model": model_repo});
    let timeout = std::time::Duration::from_secs(600);
    let rx = daemon.send_command_streaming(&cmd, timeout)?;

    // Read responses on a blocking thread, emit progress events
    let repo_clone = model_repo.clone();
    let app_for_blocking = app.clone();
    let final_resp = tokio::task::spawn_blocking(move || {
        let mut last_resp = None;
        let mut last_logged_pct: i64 = -1;
        let load_start = std::time::Instant::now();
        log::info!("[daemon] [load_model] begin repo={}", repo_clone);

        while let Ok(resp) = rx.recv() {
            if resp.status == "downloading" {
                let progress = resp.progress.as_ref()
                    .and_then(|p| p.as_f64())
                    .unwrap_or(0.0);
                let pct = (progress * 100.0) as i64;
                if pct >= last_logged_pct + 10 {
                    log::info!("[daemon] [load_model] downloading progress={}%", pct);
                    last_logged_pct = pct;
                }
                let _ = app_for_blocking.emit("model-download-progress", serde_json::json!({
                    "model_repo": repo_clone,
                    "progress": progress,
                }));
            } else if resp.status == "download_complete" {
                log::info!("[daemon] [load_model] download_complete");
            }
            let is_terminal = resp.status != "downloading"
                && resp.status != "download_complete";
            last_resp = Some(resp);
            if is_terminal { break; }
        }

        if let Some(ref resp) = last_resp {
            let elapsed = load_start.elapsed().as_millis();
            let cached = resp.cached.unwrap_or(false);
            if resp.status == "success" || resp.status == "loaded" {
                log::info!("[daemon] [load_model] loaded cached={} elapsed_ms={}", cached, elapsed);
            } else {
                log::error!("[daemon] [load_model] FAILED status={} error={:?} elapsed_ms={}",
                    resp.status, resp.error, elapsed);
            }
        }

        last_resp.ok_or_else(|| AppError::Transcription("no response from daemon".into()))
    })
    .await
    .map_err(|e| AppError::Transcription(format!("spawn_blocking: {e}")))??;

    if final_resp.status == "success" || final_resp.status == "loaded" {
        daemon.set_loaded_model(Some(model_repo));
        let _ = app.emit("daemon-status-changed", ());
        Ok(())
    } else {
        Err(AppError::Transcription(
            final_resp.error.unwrap_or_else(|| format!("Load failed: {}", final_resp.status))
        ))
    }
}

#[tauri::command]
pub fn daemon_unload_model(app: tauri::AppHandle, daemon: State<DaemonManager>) -> Result<(), AppError> {
    let cmd = serde_json::json!({"action": "unload"});
    daemon.send_command(&cmd)?;
    daemon.set_loaded_model(None);
    let _ = app.emit("daemon-status-changed", ());
    Ok(())
}

// ---------------------------------------------------------------------------
// Sprite Sheets (shared with native VoiceInk)
// ---------------------------------------------------------------------------

/// List all available sprite sheet manifests.
#[tauri::command]
pub fn list_sprites(state: State<AppContext>) -> Result<Vec<Value>, AppError> {
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
pub fn get_sprite_image(state: State<AppContext>, dir_id: String, file_name: String) -> Result<String, AppError> {
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

pub fn base64_encode(data: &[u8]) -> String {
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

/// Import a sprite from a folder selected via file dialog.
/// The folder must contain a manifest.json and the sprite image referenced within.
#[tauri::command]
pub async fn import_sprite_folder(app: AppHandle, state: State<'_, AppContext>) -> Result<Value, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let dir = app
        .dialog()
        .file()
        .blocking_pick_folder();

    let dir = match dir {
        Some(d) => d,
        None => return Ok(Value::Null), // user cancelled
    };

    let dir_path = dir
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件夹路径".into()))?;

    let manifest_path = dir_path.join("manifest.json");
    if !manifest_path.exists() {
        return Err(AppError::Io("文件夹中未找到 manifest.json".into()));
    }

    let manifest_data = std::fs::read_to_string(&manifest_path)?;
    let manifest: Value = serde_json::from_str(&manifest_data)
        .map_err(|e| AppError::Io(format!("manifest.json 解析失败: {}", e)))?;

    let sprite_file = manifest
        .get("spriteFile")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Io("manifest.json 缺少 spriteFile 字段".into()))?;

    let sprite_path = dir_path.join(sprite_file);
    if !sprite_path.exists() {
        return Err(AppError::Io(format!("精灵图文件不存在: {}", sprite_file)));
    }

    // Create destination subdirectory
    let dest_id = uuid::Uuid::new_v4().to_string().to_uppercase();
    let dest_dir = state.paths.sprites_dir.join(&dest_id);
    std::fs::create_dir_all(&dest_dir)?;

    // Copy manifest.json
    std::fs::copy(&manifest_path, dest_dir.join("manifest.json"))?;
    // Copy sprite image
    std::fs::copy(&sprite_path, dest_dir.join(sprite_file))?;
    // Copy sprite_processed.png if it exists
    let processed = dir_path.join("sprite_processed.png");
    if processed.exists() {
        std::fs::copy(&processed, dest_dir.join("sprite_processed.png"))?;
    }

    info!("[sprite] imported folder {:?} -> {}", dir_path, dest_id);

    let mut result = manifest.clone();
    result.as_object_mut().map(|m| m.insert("dirId".into(), Value::String(dest_id)));
    Ok(result)
}

/// Import a sprite from a .zip archive selected via file dialog.
#[tauri::command]
pub async fn import_sprite_zip(app: AppHandle, state: State<'_, AppContext>) -> Result<Value, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("Sprite Archive", &["zip"])
        .blocking_pick_file();

    let file = match file {
        Some(f) => f,
        None => return Ok(Value::Null),
    };

    let file_path = file
        .as_path()
        .ok_or_else(|| AppError::Io("无法获取文件路径".into()))?;

    // Extract to temp directory
    let tmp_dir = std::env::temp_dir().join(format!("sprite-import-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir)?;

    let zip_file = std::fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| AppError::Io(format!("无法打开 zip 文件: {}", e)))?;
    archive.extract(&tmp_dir)
        .map_err(|e| AppError::Io(format!("解压失败: {}", e)))?;

    // Find manifest.json (may be in root or a subdirectory)
    let manifest_path = find_manifest_in_dir(&tmp_dir)
        .ok_or_else(|| AppError::Io("zip 中未找到 manifest.json".into()))?;

    let manifest_data = std::fs::read_to_string(&manifest_path)?;
    let manifest: Value = serde_json::from_str(&manifest_data)
        .map_err(|e| AppError::Io(format!("manifest.json 解析失败: {}", e)))?;

    let sprite_file = manifest
        .get("spriteFile")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Io("manifest.json 缺少 spriteFile 字段".into()))?;

    let manifest_dir = manifest_path.parent().unwrap_or(&tmp_dir);
    let sprite_path = manifest_dir.join(sprite_file);
    if !sprite_path.exists() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(AppError::Io(format!("精灵图文件不存在: {}", sprite_file)));
    }

    // Create destination
    let dest_id = uuid::Uuid::new_v4().to_string().to_uppercase();
    let dest_dir = state.paths.sprites_dir.join(&dest_id);
    std::fs::create_dir_all(&dest_dir)?;

    std::fs::copy(&manifest_path, dest_dir.join("manifest.json"))?;
    std::fs::copy(&sprite_path, dest_dir.join(sprite_file))?;

    // Copy processed file if present
    let processed = manifest_dir.join("sprite_processed.png");
    if processed.exists() {
        std::fs::copy(&processed, dest_dir.join("sprite_processed.png"))?;
    }

    // Cleanup temp
    let _ = std::fs::remove_dir_all(&tmp_dir);

    info!("[sprite] imported zip {:?} -> {}", file_path, dest_id);

    let mut result = manifest.clone();
    result.as_object_mut().map(|m| m.insert("dirId".into(), Value::String(dest_id)));
    Ok(result)
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

/// Delete a sprite sheet by its directory ID.
#[tauri::command]
pub fn delete_sprite(state: State<AppContext>, dir_id: String) -> Result<(), AppError> {
    let dir = state.paths.sprites_dir.join(&dir_id);
    if dir.exists() && dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
        info!("[sprite] deleted {}", dir_id);
    }
    // Clear selected_sprite_id if it was the deleted one
    {
        let conn = state.db.lock().map_err(|e| AppError::Database(e.to_string()))?;
        if let Ok(Some(current)) = db::get_setting(&conn, "selected_sprite_id") {
            if current.as_str() == Some(dir_id.as_str()) {
                drop(conn);
                let _ = state.set_setting_cached("selected_sprite_id", &serde_json::Value::String(String::new()));
            }
        }
    }
    Ok(())
}

/// Process a sprite sheet image: remove background color and save as sprite_processed.png.
#[tauri::command]
pub fn process_sprite_background(
    state: State<AppContext>,
    dir_id: String,
    threshold: f64,
) -> Result<(), AppError> {
    let dir = state.paths.sprites_dir.join(&dir_id);
    let manifest_path = dir.join("manifest.json");
    let manifest_data = std::fs::read_to_string(&manifest_path)?;
    let manifest: Value = serde_json::from_str(&manifest_data)
        .map_err(|e| AppError::Io(format!("manifest.json parse error: {}", e)))?;

    let sprite_file = manifest
        .get("spriteFile")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Io("missing spriteFile".into()))?;

    let src_path = dir.join(sprite_file);
    let mut img = image::open(&src_path)
        .map_err(|e| AppError::Io(format!("failed to open image: {}", e)))?
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

    let samples: Vec<(f64, f64, f64)> = corners.iter().map(|&(cx, cy)| {
        let p = img.get_pixel(cx, cy);
        (p[0] as f64 / 255.0, p[1] as f64 / 255.0, p[2] as f64 / 255.0)
    }).collect();

    // Pick two most similar
    let mut best_dist = f64::MAX;
    let (mut bi, mut bj) = (0usize, 1usize);
    for i in 0..4 {
        for j in (i + 1)..4 {
            let d = color_dist(&samples[i], &samples[j]);
            if d < best_dist { best_dist = d; bi = i; bj = j; }
        }
    }

    let bg = (
        (samples[bi].0 + samples[bj].0) / 2.0,
        (samples[bi].1 + samples[bj].1) / 2.0,
        (samples[bi].2 + samples[bj].2) / 2.0,
    );

    info!("[sprite] process_background: {}x{} bg=({:.3},{:.3},{:.3}) threshold={}",
          width, height, bg.0, bg.1, bg.2, threshold);

    for pixel in img.pixels_mut() {
        let c = (pixel[0] as f64 / 255.0, pixel[1] as f64 / 255.0, pixel[2] as f64 / 255.0);
        if color_dist(&c, &bg) < threshold {
            pixel[0] = 0; pixel[1] = 0; pixel[2] = 0; pixel[3] = 0;
        }
    }

    let dest_path = dir.join("sprite_processed.png");
    img.save(&dest_path)
        .map_err(|e| AppError::Io(format!("failed to save processed image: {}", e)))?;

    info!("[sprite] process_background: saved {}", dest_path.display());
    Ok(())
}

fn color_dist(a: &(f64, f64, f64), b: &(f64, f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
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

// ---------------------------------------------------------------------------
// System locale
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
}
