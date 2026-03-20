use std::collections::HashMap;
use serde_json::Value;
use yumo_lib::state::AppPaths;

#[test]
fn test_app_paths_defaults() {
    let paths = AppPaths::defaults();
    let home = dirs::home_dir().unwrap();

    assert_eq!(paths.data_dir, home.join(".voiceink"));
    assert_eq!(paths.models_dir, home.join(".voiceink/models"));
    assert_eq!(
        paths.sprites_dir,
        home.join("Library/Application Support/VoiceInk/SpriteSheets")
    );
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

    assert_eq!(paths.data_dir.to_str().unwrap(), "/tmp/voiceink-test");
    // models_dir should follow data_dir when not explicitly set
    assert_eq!(paths.models_dir.to_str().unwrap(), "/tmp/voiceink-test/models");
}

#[test]
fn test_app_paths_override_models_dir() {
    let mut settings = HashMap::new();
    settings.insert("path_models".to_string(), Value::String("/opt/models".to_string()));

    let paths = AppPaths::from_settings(&settings);

    // data_dir stays default, models_dir is overridden
    let home = dirs::home_dir().unwrap();
    assert_eq!(paths.data_dir, home.join(".voiceink"));
    assert_eq!(paths.models_dir.to_str().unwrap(), "/opt/models");
}

#[test]
fn test_app_paths_override_sprites_dir() {
    let mut settings = HashMap::new();
    settings.insert("path_sprites".to_string(), Value::String("/my/sprites".to_string()));

    let paths = AppPaths::from_settings(&settings);
    assert_eq!(paths.sprites_dir.to_str().unwrap(), "/my/sprites");
}

#[test]
fn test_app_paths_override_all() {
    let mut settings = HashMap::new();
    settings.insert("path_data".to_string(), Value::String("/a".to_string()));
    settings.insert("path_models".to_string(), Value::String("/b".to_string()));
    settings.insert("path_sprites".to_string(), Value::String("/c".to_string()));

    let paths = AppPaths::from_settings(&settings);

    assert_eq!(paths.data_dir.to_str().unwrap(), "/a");
    assert_eq!(paths.models_dir.to_str().unwrap(), "/b");
    assert_eq!(paths.sprites_dir.to_str().unwrap(), "/c");
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
