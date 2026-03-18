pub mod audio_ctrl;
pub mod cloud;
pub mod commands;
pub mod db;
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(state::AppState::new(conn))
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_transcriptions,
            commands::delete_transcription,
            commands::delete_all_transcriptions,
            commands::get_vocabulary,
            commands::add_vocabulary,
            commands::delete_vocabulary,
            commands::get_replacements,
            commands::set_replacement,
            commands::delete_replacement,
            commands::get_settings,
            commands::update_setting,
            commands::list_prompts,
            commands::add_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            commands::select_prompt,
            commands::select_model,
            commands::store_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::register_hotkey,
            commands::unregister_hotkey,
            commands::import_dictionary_csv,
            commands::export_dictionary_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
