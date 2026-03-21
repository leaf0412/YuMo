use yumo_core::db;
use yumo_core::platform::AudioData;

#[test]
fn test_transcription_stores_recording_path() {
    let conn = db::init_database(std::path::Path::new(":memory:")).unwrap();
    let id = db::insert_transcription(
        &conn, "hello", None, 1.0, "test-model", 1,
        Some("/tmp/recording_test.wav"),
    ).unwrap();

    let result = db::get_transcriptions(&conn, None, None, 10).unwrap();
    let item = result.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.recording_path.as_deref(), Some("/tmp/recording_test.wav"));
}

#[test]
fn test_transcription_without_recording_path() {
    let conn = db::init_database(std::path::Path::new(":memory:")).unwrap();
    let id = db::insert_transcription(
        &conn, "hello", None, 1.0, "test-model", 1, None,
    ).unwrap();

    let result = db::get_transcriptions(&conn, None, None, 10).unwrap();
    let item = result.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.recording_path, None);
}

#[test]
fn test_get_recording_audio_returns_base64() {
    let tmp = tempfile::TempDir::new().unwrap();
    let audio = AudioData {
        pcm_samples: vec![0.1f32; 16000],
        sample_rate: 16000,
        channels: 1,
    };
    let path = yumo_core::audio_io::save_recording(&audio, tmp.path()).unwrap();

    let data_uri = yumo_core::audio_io::read_recording_as_data_uri(&path).unwrap();
    assert!(data_uri.starts_with("data:audio/wav;base64,"));
    assert!(data_uri.len() > 100);
}

#[test]
fn test_get_recording_audio_not_found() {
    let result = yumo_core::audio_io::read_recording_as_data_uri(
        std::path::Path::new("/nonexistent/file.wav")
    );
    assert!(result.is_err());
}
