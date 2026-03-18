use voiceink_tauri_lib::audio_ctrl;

#[test]
fn test_get_default_output_device() {
    let device_id = audio_ctrl::default_output_device_id();
    assert_ne!(device_id, 0, "Should find a default output device");
}

#[test]
fn test_mute_unmute_roundtrip() {
    let original = audio_ctrl::is_system_muted();

    audio_ctrl::set_system_muted(true);
    assert!(audio_ctrl::is_system_muted());

    audio_ctrl::set_system_muted(false);
    assert!(!audio_ctrl::is_system_muted());

    // Restore
    audio_ctrl::set_system_muted(original);
}
