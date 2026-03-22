use yumo_core::platform::recorder;
use yumo_core::platform::AudioData;
use yumo_core::audio_io;

#[test]
fn test_list_audio_devices() {
    // CI 可能无音频设备，只验证不 panic
    let result = recorder::list_input_devices();
    if let Ok(devices) = result {
        for dev in &devices {
            assert!(!dev.name.is_empty());
            assert!(dev.id > 0);
        }
    }
}

// Recording tests need microphone permission — mark as ignore for CI
#[test]
#[ignore]
fn test_record_short_audio() {
    let devices = recorder::list_input_devices().unwrap();
    let device_id = devices[0].id;

    let (handle, _rx) = recorder::start_recording(device_id).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let audio_data = recorder::stop_recording(handle).unwrap();

    assert!(audio_data.pcm_samples.len() > 1000);
    assert_eq!(audio_data.sample_rate, 16000);
    assert_eq!(audio_data.channels, 1);
}

#[test]
fn test_save_wav_from_synthetic_data() {
    // Test WAV writing with synthetic data (no mic needed)
    let audio_data = AudioData {
        pcm_samples: vec![0.0f32; 16000], // 1 second of silence
        sample_rate: 16000,
        channels: 1,
    };

    let tmp = tempfile::TempDir::new().unwrap();
    let wav_path = tmp.path().join("test.wav");
    audio_io::save_wav(&audio_data, &wav_path).unwrap();

    assert!(wav_path.exists());
    let metadata = std::fs::metadata(&wav_path).unwrap();
    assert!(metadata.len() > 44); // WAV header is 44 bytes minimum
}

#[test]
fn test_save_wav_readable() {
    let audio_data = AudioData {
        pcm_samples: (0..16000).map(|i| (i as f32 * 0.001).sin()).collect(),
        sample_rate: 16000,
        channels: 1,
    };

    let tmp = tempfile::TempDir::new().unwrap();
    let wav_path = tmp.path().join("sine.wav");
    audio_io::save_wav(&audio_data, &wav_path).unwrap();

    // Read back with hound and verify
    let reader = hound::WavReader::open(&wav_path).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 16000);
}
