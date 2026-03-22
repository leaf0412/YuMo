use yumo_lib::state::AppPaths;
use yumo_lib::platform::AudioData;

// ---------------------------------------------------------------------------
// AppPaths: recordings_dir
// ---------------------------------------------------------------------------

#[test]
fn test_default_recordings_dir() {
    let paths = AppPaths::defaults();
    let home = dirs::home_dir().unwrap();
    assert_eq!(paths.recordings_dir, home.join(".voiceink").join("recordings"));
}

#[test]
fn test_recordings_dir_follows_data_dir() {
    let mut settings = std::collections::HashMap::new();
    settings.insert("path_data".into(), serde_json::Value::String("/tmp/vi".into()));
    let paths = AppPaths::from_settings(&settings);
    assert_eq!(paths.recordings_dir, std::path::PathBuf::from("/tmp/vi").join("recordings"));
}

#[test]
fn test_recordings_dir_override() {
    let mut settings = std::collections::HashMap::new();
    settings.insert("path_recordings".into(), serde_json::Value::String("/my/recs".into()));
    let paths = AppPaths::from_settings(&settings);
    assert_eq!(paths.recordings_dir, std::path::PathBuf::from("/my/recs"));
}

// ---------------------------------------------------------------------------
// WAV file saving
// ---------------------------------------------------------------------------

#[test]
fn test_save_recording_creates_wav_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    let audio = AudioData {
        pcm_samples: vec![0.0f32; 16000], // 1 second of silence
        sample_rate: 16000,
        channels: 1,
    };

    let path = yumo_lib::audio_io::save_recording(&audio, &dir).unwrap();

    assert!(path.exists());
    assert!(path.extension().unwrap() == "wav");
    assert!(path.parent().unwrap() == dir);
    // File should be non-empty
    assert!(std::fs::metadata(&path).unwrap().len() > 44); // WAV header = 44 bytes
}

#[test]
fn test_save_recording_creates_dir_if_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("sub/dir");

    let audio = AudioData {
        pcm_samples: vec![0.1f32; 8000],
        sample_rate: 16000,
        channels: 1,
    };

    let path = yumo_lib::audio_io::save_recording(&audio, &dir).unwrap();
    assert!(path.exists());
}

#[test]
fn test_save_recording_unique_filenames() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    let audio = AudioData {
        pcm_samples: vec![0.0f32; 1600],
        sample_rate: 16000,
        channels: 1,
    };

    let path1 = yumo_lib::audio_io::save_recording(&audio, &dir).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let path2 = yumo_lib::audio_io::save_recording(&audio, &dir).unwrap();

    assert_ne!(path1, path2);
}

#[test]
fn test_save_recording_filename_format() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    let audio = AudioData {
        pcm_samples: vec![0.0f32; 1600],
        sample_rate: 16000,
        channels: 1,
    };

    let path = yumo_lib::audio_io::save_recording(&audio, &dir).unwrap();
    let name = path.file_stem().unwrap().to_str().unwrap();
    // Should start with "recording_" followed by a timestamp
    assert!(name.starts_with("recording_"), "filename should start with recording_, got: {}", name);
}
