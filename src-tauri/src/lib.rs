pub mod audio_ctrl;
pub mod commands;
pub mod db;
pub mod downloader;
pub mod enhancer;
pub mod error;
pub mod keychain;
pub mod paster;
pub mod permissions;
pub mod pipeline;
pub mod state;
pub mod text_processor;

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
        .manage(state::AppState::new(conn))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
