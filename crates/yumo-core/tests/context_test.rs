//! Tests AppContext construction and field access.

use yumo_core::pipeline::PipelineState;
use yumo_core::state::{AppContext, AppPaths};

#[test]
fn test_app_context_construction() {
    let expected_paths = AppPaths::defaults();

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    // Manually create the tables since init_database expects a file path
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transcriptions (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            enhanced_text TEXT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            duration REAL NOT NULL,
            model_name TEXT NOT NULL,
            word_count INTEGER NOT NULL,
            recording_path TEXT
        );"
    ).unwrap();

    let paths = AppPaths::defaults();
    let ctx = AppContext::new(conn, paths);

    // Verify initial pipeline state is Idle
    let pipeline = ctx.pipeline_state.lock().unwrap();
    assert!(matches!(*pipeline, PipelineState::Idle));
    drop(pipeline);

    // Verify recording_handle starts as None
    let handle = ctx.recording_handle.lock().unwrap();
    assert!(handle.is_none());
    drop(handle);

    // Verify denoiser starts as None
    let denoiser = ctx.denoiser.lock().unwrap();
    assert!(denoiser.is_none());
    drop(denoiser);

    // Verify paths are preserved
    assert_eq!(ctx.paths.data_dir, expected_paths.data_dir);
    assert_eq!(ctx.paths.models_dir, expected_paths.models_dir);
    assert_eq!(ctx.paths.sprites_dir, expected_paths.sprites_dir);
    assert_eq!(ctx.paths.recordings_dir, expected_paths.recordings_dir);
    assert_eq!(ctx.paths.denoiser_dir, expected_paths.denoiser_dir);
}

#[test]
fn test_app_context_db_access() {
    let paths = AppPaths::defaults();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    ).unwrap();

    let ctx = AppContext::new(conn, paths);

    // Verify DB is accessible through the Mutex
    let db = ctx.db.lock().unwrap();
    db.execute("INSERT INTO settings (key, value) VALUES (?1, ?2)", rusqlite::params!["test_key", "test_value"]).unwrap();

    let val: String = db.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params!["test_key"],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(val, "test_value");
}

#[test]
fn test_app_context_pipeline_state_mutation() {
    let paths = AppPaths::defaults();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let ctx = AppContext::new(conn, paths);

    // Mutate pipeline state through the Mutex
    {
        let mut pipeline = ctx.pipeline_state.lock().unwrap();
        *pipeline = PipelineState::Recording;
    }

    let pipeline = ctx.pipeline_state.lock().unwrap();
    assert_eq!(*pipeline, PipelineState::Recording);
}

#[test]
fn test_app_paths_from_settings_custom_data_dir() {
    use std::collections::HashMap;

    let mut settings = HashMap::new();
    settings.insert(
        "path_data".to_string(),
        serde_json::json!("/tmp/yumo-test"),
    );
    let paths = AppPaths::from_settings(&settings);
    assert_eq!(paths.data_dir.to_string_lossy(), "/tmp/yumo-test");
    // Derived dirs should follow data_dir
    assert!(paths
        .models_dir
        .to_string_lossy()
        .contains("/tmp/yumo-test/models"));
    assert!(paths
        .recordings_dir
        .to_string_lossy()
        .contains("/tmp/yumo-test/recordings"));
    assert!(paths
        .denoiser_dir
        .to_string_lossy()
        .contains("/tmp/yumo-test/denoiser"));
}
