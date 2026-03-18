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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs::home_dir()
        .expect("No home directory")
        .join(".voiceink");
    std::fs::create_dir_all(&data_dir).expect("Cannot create data dir");

    let db_path = data_dir.join("data.db");
    let conn = db::init_database(&db_path).expect("Cannot init database");

    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir).expect("Cannot create models dir");

    // Always sync daemon script from source to data_dir (handles updates)
    let daemon_script = data_dir.join("mlx_funasr_daemon.py");
    let dev_script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources/mlx_funasr_daemon.py");
    if dev_script.exists() {
        let _ = std::fs::copy(&dev_script, &daemon_script);
    }
    let daemon = daemon::DaemonManager::new(daemon_script, data_dir.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(state::AppState::new(conn, models_dir, daemon))
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Recording pipeline
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
