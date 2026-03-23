use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::{json, Value};
use yumo_core::state::{AppContext, AppPaths};
use yumo_core::db;

#[test]
fn test_app_paths_defaults() {
    let paths = AppPaths::defaults();
    let home = dirs::home_dir().unwrap();
    let vi = home.join(".voiceink");

    assert_eq!(paths.data_dir, vi);
    assert_eq!(paths.models_dir, vi.join("models"));
    assert_eq!(paths.sprites_dir, vi.join("sprites"));
}

#[test]
fn test_app_paths_from_empty_settings() {
    let settings = HashMap::new();
    let paths = AppPaths::from_settings(&settings);
    let defaults = AppPaths::defaults();

    assert_eq!(paths.data_dir, defaults.data_dir);
    assert_eq!(paths.models_dir, defaults.models_dir);
    assert_eq!(paths.sprites_dir, defaults.sprites_dir);
}

#[test]
fn test_app_paths_override_data_dir() {
    let mut settings = HashMap::new();
    settings.insert("path_data".to_string(), Value::String("/tmp/voiceink-test".to_string()));

    let paths = AppPaths::from_settings(&settings);
    let base = PathBuf::from("/tmp/voiceink-test");

    assert_eq!(paths.data_dir, base);
    // models_dir should follow data_dir when not explicitly set
    assert_eq!(paths.models_dir, base.join("models"));
}

#[test]
fn test_app_paths_override_models_dir() {
    let mut settings = HashMap::new();
    settings.insert("path_models".to_string(), Value::String("/opt/models".to_string()));

    let paths = AppPaths::from_settings(&settings);

    // data_dir stays default, models_dir is overridden
    let home = dirs::home_dir().unwrap();
    assert_eq!(paths.data_dir, home.join(".voiceink"));
    assert_eq!(paths.models_dir, PathBuf::from("/opt/models"));
}

#[test]
fn test_app_paths_override_sprites_dir() {
    let mut settings = HashMap::new();
    settings.insert("path_sprites".to_string(), Value::String("/my/sprites".to_string()));

    let paths = AppPaths::from_settings(&settings);
    assert_eq!(paths.sprites_dir, PathBuf::from("/my/sprites"));
}

#[test]
fn test_app_paths_override_all() {
    let mut settings = HashMap::new();
    settings.insert("path_data".to_string(), Value::String("/a".to_string()));
    settings.insert("path_models".to_string(), Value::String("/b".to_string()));
    settings.insert("path_sprites".to_string(), Value::String("/c".to_string()));

    let paths = AppPaths::from_settings(&settings);

    assert_eq!(paths.data_dir, PathBuf::from("/a"));
    assert_eq!(paths.models_dir, PathBuf::from("/b"));
    assert_eq!(paths.sprites_dir, PathBuf::from("/c"));
}

#[test]
fn test_app_paths_ignores_non_string_values() {
    let mut settings = HashMap::new();
    settings.insert("path_data".to_string(), Value::Number(serde_json::Number::from(42)));
    settings.insert("path_sprites".to_string(), Value::Bool(true));

    let paths = AppPaths::from_settings(&settings);
    let defaults = AppPaths::defaults();

    // Should fall back to defaults for non-string values
    assert_eq!(paths.data_dir, defaults.data_dir);
    assert_eq!(paths.sprites_dir, defaults.sprites_dir);
}

#[test]
fn test_settings_cache_initialized_and_readable() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let paths = AppPaths {
        data_dir: PathBuf::from("/tmp/test-data"),
        models_dir: PathBuf::from("/tmp/test-data/models"),
        sprites_dir: PathBuf::from("/tmp/test-data/sprites"),
        recordings_dir: PathBuf::from("/tmp/test-data/recordings"),
    };

    let mut initial = HashMap::new();
    initial.insert("language".to_string(), Value::String("zh".to_string()));
    initial.insert("model".to_string(), Value::String("base".to_string()));

    let ctx = AppContext::new(conn, paths, initial);

    let cache = ctx.settings_cache.read().unwrap();
    assert_eq!(cache.get("language").and_then(|v| v.as_str()), Some("zh"));
    assert_eq!(cache.get("model").and_then(|v| v.as_str()), Some("base"));
    assert_eq!(cache.len(), 2);
}

#[test]
fn test_settings_cache_write_and_read() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let paths = AppPaths {
        data_dir: PathBuf::from("/tmp/test-data"),
        models_dir: PathBuf::from("/tmp/test-data/models"),
        sprites_dir: PathBuf::from("/tmp/test-data/sprites"),
        recordings_dir: PathBuf::from("/tmp/test-data/recordings"),
    };

    let ctx = AppContext::new(conn, paths, HashMap::new());

    // Write to cache
    {
        let mut cache = ctx.settings_cache.write().unwrap();
        cache.insert("hotkey".to_string(), Value::String("Cmd+Shift+R".to_string()));
    }

    // Read back
    let cache = ctx.settings_cache.read().unwrap();
    assert_eq!(
        cache.get("hotkey").and_then(|v| v.as_str()),
        Some("Cmd+Shift+R")
    );
}

/// Helper: create an in-memory DB with the settings table.
fn in_memory_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .unwrap();
    conn
}

fn test_paths() -> AppPaths {
    AppPaths {
        data_dir: PathBuf::from("/tmp/test-data"),
        models_dir: PathBuf::from("/tmp/test-data/models"),
        sprites_dir: PathBuf::from("/tmp/test-data/sprites"),
        recordings_dir: PathBuf::from("/tmp/test-data/recordings"),
    }
}

#[test]
fn test_set_setting_cached_syncs_db_and_cache() {
    let conn = in_memory_db();
    let ctx = AppContext::new(conn, test_paths(), HashMap::new());

    // Call set_setting_cached
    ctx.set_setting_cached("test_key", &json!("hello")).unwrap();

    // Verify cache has the value
    {
        let cache = ctx.settings_cache.read().unwrap();
        assert_eq!(cache.get("test_key"), Some(&json!("hello")));
    }

    // Verify DB has the value
    {
        let conn = ctx.db.lock().unwrap();
        let all = db::get_all_settings(&conn).unwrap();
        assert_eq!(all.get("test_key"), Some(&json!("hello")));
    }
}

#[test]
fn test_set_setting_cached_overwrites_existing() {
    let conn = in_memory_db();
    let mut initial = HashMap::new();
    initial.insert("lang".to_string(), json!("en"));
    let ctx = AppContext::new(conn, test_paths(), initial);

    // Seed DB so it matches cache
    {
        let conn = ctx.db.lock().unwrap();
        db::update_setting(&conn, "lang", &json!("en")).unwrap();
    }

    // Overwrite
    ctx.set_setting_cached("lang", &json!("zh")).unwrap();

    let cache = ctx.settings_cache.read().unwrap();
    assert_eq!(cache.get("lang"), Some(&json!("zh")));

    let conn = ctx.db.lock().unwrap();
    let all = db::get_all_settings(&conn).unwrap();
    assert_eq!(all.get("lang"), Some(&json!("zh")));
}
