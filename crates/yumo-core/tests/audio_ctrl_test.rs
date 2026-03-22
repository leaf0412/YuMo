use yumo_core::platform::audio_ctrl;

#[test]
fn test_is_system_muted_returns_ok() {
    // Should return Ok with a bool, not crash
    let result = audio_ctrl::is_system_muted();
    assert!(result.is_ok(), "is_system_muted should return Ok, got {:?}", result);
}

#[test]
#[ignore] // requires real audio device — skip in headless CI
fn test_mute_unmute_roundtrip() {
    let original = audio_ctrl::is_system_muted().unwrap();

    audio_ctrl::set_system_muted(true).unwrap();
    assert!(audio_ctrl::is_system_muted().unwrap());

    audio_ctrl::set_system_muted(false).unwrap();
    assert!(!audio_ctrl::is_system_muted().unwrap());

    // Restore
    audio_ctrl::set_system_muted(original).unwrap();
}
