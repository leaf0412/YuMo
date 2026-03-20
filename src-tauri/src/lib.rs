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
pub mod mask;
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
use tauri::Emitter;
use state::AppPaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Bootstrap: use default data_dir for DB + logger init
    let defaults = AppPaths::defaults();
    std::fs::create_dir_all(&defaults.data_dir).expect("Cannot create data dir");

    // Init file logger
    let startup_start = std::time::Instant::now();
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
    info!("[app] [startup] version={} os={} arch={} log={}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH, log_path.display());

    // Init DB
    let db_path = defaults.data_dir.join("data.db");
    let conn = db::init_database(&db_path).expect("Cannot init database");
    info!("[db] [init] path={} elapsed_ms={}", db_path.display(), startup_start.elapsed().as_millis());

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

    // Daemon script path (will be synced from resources in setup())
    let daemon_script = paths.data_dir.join("mlx_funasr_daemon.py");
    let daemon = daemon::DaemonManager::new(daemon_script, paths.data_dir.clone());
    info!("[app] [startup_complete] elapsed_ms={}", startup_start.elapsed().as_millis());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(state::AppState::new(conn, paths, daemon))
        .setup(move |app| {
            // Sync bundled resources (daemon script + uv) to ~/.voiceink/
            // Uses Tauri's resource_dir() which works in both dev and production builds.
            {
                use tauri::Manager;
                let app_state = app.handle().state::<state::AppState>();
                let data_dir = &app_state.paths.data_dir;

                // Try Tauri resource resolver first (production), fall back to CARGO_MANIFEST_DIR (dev)
                let res_dir = app.path().resource_dir()
                    .ok()
                    .map(|d| d.join("resources"));
                let dev_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");

                let sync_file = |name: &str, executable: bool| {
                    let dest = data_dir.join(name);
                    // Try production path first, then dev path
                    let src = res_dir.as_ref()
                        .map(|d| d.join(name))
                        .filter(|p| p.exists())
                        .unwrap_or_else(|| dev_dir.join(name));

                    if !src.exists() {
                        info!("[app] [sync_resource] {} not found at {:?}", name, src);
                        return;
                    }

                    let src_size = std::fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
                    let dest_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                    if !dest.exists() || src_size != dest_size {
                        match std::fs::copy(&src, &dest) {
                            Ok(_) => {
                                info!("[app] [sync_resource] {} synced ({} bytes)", name, src_size);
                                #[cfg(unix)]
                                if executable {
                                    use std::os::unix::fs::PermissionsExt;
                                    let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
                                }
                            }
                            Err(e) => log::error!("[app] [sync_resource] {} copy failed: {}", name, e),
                        }
                    } else {
                        info!("[app] [sync_resource] {} up to date", name);
                    }
                };

                sync_file("mlx_funasr_daemon.py", false);
                sync_file("uv", true);
            }

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
                                        let _ = handle.emit("daemon-status-changed", ());
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
            commands::get_recording,
            commands::delete_transcription,
            commands::delete_all_transcriptions,
            commands::get_statistics,
            // Import
            commands::import_voiceink_legacy,
            commands::detect_voiceink_legacy_path,
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
