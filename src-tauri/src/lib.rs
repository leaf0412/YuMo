pub mod audio_ctrl;
pub mod cloud;
pub mod commands;
pub mod daemon;
pub mod db;
pub mod denoiser;
pub mod downloader;
pub mod enhancer;
pub mod error;
pub mod hotkey;
pub mod keychain;
pub mod paster;
pub mod permissions;
pub mod pipeline;
pub mod recorder;
pub mod state;
pub mod text_processor;
pub mod tray;
pub mod transcriber;
pub mod vad;
pub mod window_manager;

use log::info;
use state::AppPaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Bootstrap: use default data_dir for DB + logger init
    let defaults = AppPaths::defaults();
    std::fs::create_dir_all(&defaults.data_dir).expect("Cannot create data dir");

    // Init file logger
    let log_path = defaults.data_dir.join("log.txt");
    let log_file = std::fs::File::create(&log_path).expect("Cannot create log file");
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            log_file,
        ),
    ])
    .expect("Cannot init logger");
    info!("VoiceInk starting, log file: {}", log_path.display());

    // Init DB
    let db_path = defaults.data_dir.join("data.db");
    let conn = db::init_database(&db_path).expect("Cannot init database");

    // Read settings to build paths (overrides from DB)
    let saved_settings = db::get_all_settings(&conn).unwrap_or_default();
    let paths = AppPaths::from_settings(&saved_settings);
    let saved_hotkey = saved_settings
        .get("hotkey")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    info!("Paths: data={} models={} sprites={}",
        paths.data_dir.display(), paths.models_dir.display(), paths.sprites_dir.display());

    std::fs::create_dir_all(&paths.models_dir).expect("Cannot create models dir");

    // Sync daemon script
    let daemon_script = paths.data_dir.join("mlx_funasr_daemon.py");
    let dev_script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources/mlx_funasr_daemon.py");
    if dev_script.exists() {
        let _ = std::fs::copy(&dev_script, &daemon_script);
    }
    let daemon = daemon::DaemonManager::new(daemon_script, paths.data_dir.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(state::AppState::new(conn, paths, daemon))
        .setup(move |app| {
            tray::setup_tray(app.handle())?;

            // Configure recorder window for transparent dragging on macOS
            #[cfg(target_os = "macos")]
            {
                use tauri::Manager;
                if let Some(win) = app.get_webview_window("recorder") {
                    match win.ns_window() {
                        Ok(raw) => {
                            use objc::{msg_send, sel, sel_impl};
                            let ptr = raw as cocoa::base::id;
                            unsafe {
                                let _: () = msg_send![ptr, setIgnoresMouseEvents: false];
                                let _: () = msg_send![ptr, setMovableByWindowBackground: true];
                            }
                            info!("[wm] recorder NSWindow configured for mouse events");
                        }
                        Err(e) => info!("[wm] ns_window() failed: {:?}", e),
                    }
                }
            }

            // Restore saved hotkey
            if let Some(shortcut) = &saved_hotkey {
                let handle = app.handle().clone();
                match hotkey::register_shortcut(app.handle(), shortcut, move || {
                    use tauri::Emitter;
                    info!("[hotkey] triggered! emitting toggle-recording");
                    let _ = handle.emit("toggle-recording", ());
                }) {
                    Ok(()) => info!("Restored hotkey: {}", shortcut),
                    Err(e) => info!("Failed to restore hotkey {}: {}", shortcut, e),
                }
            }

            // Pre-warm MLX daemon in background if a downloaded MLX model is selected
            {
                use tauri::Manager;
                let app_state = app.handle().state::<state::AppState>();
                let selected_model = saved_settings
                    .get("selected_model_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut warmup_repo: Option<String> = None;
                if !selected_model.is_empty() {
                    let all = transcriber::all_models(&app_state.paths.models_dir);
                    if let Some(model) = all.iter().find(|m| m.id == selected_model) {
                        if let transcriber::ModelProvider::MlxFunASR = model.provider {
                            if let Some(repo) = &model.model_repo {
                                if transcriber::check_mlx_model_downloaded(repo) {
                                    warmup_repo = Some(repo.clone());
                                }
                            }
                        }
                    }
                }

                if let Some(repo) = warmup_repo {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        let state = handle.state::<state::AppState>();
                        info!("[warmup] starting daemon for MLX model: {}", repo);
                        match state.daemon.start() {
                            Ok(()) => {
                                info!("[warmup] daemon started, loading model...");
                                let cmd = serde_json::json!({"action": "load", "model": &repo});
                                match state.daemon.send_command(&cmd) {
                                    Ok(resp) if resp.status == "success" || resp.status == "loaded" || resp.status == "download_complete" => {
                                        state.daemon.set_loaded_model(Some(repo.clone()));
                                        info!("[warmup] model loaded: {}", repo);
                                    }
                                    Ok(resp) => info!("[warmup] load response: {}", resp.status),
                                    Err(e) => info!("[warmup] load failed: {}", e),
                                }
                            }
                            Err(e) => info!("[warmup] daemon start failed: {}", e),
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Log bridge
            commands::frontend_log,
            // Recording pipeline
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::get_pipeline_state,
            commands::list_audio_devices,
            commands::check_permissions,
            commands::request_permission,
            // Model management
            commands::list_available_models,
            commands::download_model,
            commands::delete_model,
            commands::import_model,
            // Transcriptions
            commands::get_transcriptions,
            commands::delete_transcription,
            commands::delete_all_transcriptions,
            // Vocabulary
            commands::get_vocabulary,
            commands::add_vocabulary,
            commands::delete_vocabulary,
            // Replacements
            commands::get_replacements,
            commands::set_replacement,
            commands::delete_replacement,
            // Settings
            commands::get_settings,
            commands::update_setting,
            // Prompts
            commands::list_prompts,
            commands::add_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            commands::select_prompt,
            commands::select_model,
            // Keychain
            commands::store_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            // Hotkey
            commands::register_hotkey,
            commands::unregister_hotkey,
            // CSV
            commands::import_dictionary_csv,
            commands::export_dictionary_csv,
            // MLX Daemon
            commands::daemon_start,
            commands::daemon_stop,
            commands::daemon_status,
            commands::daemon_check_deps,
            commands::daemon_load_model,
            commands::daemon_unload_model,
            // Sprite sheets
            commands::list_sprites,
            commands::get_sprite_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
