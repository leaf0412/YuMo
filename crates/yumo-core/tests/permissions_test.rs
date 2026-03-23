use yumo_core::platform::permissions;
use yumo_core::platform::PermissionStatus;

#[test]
fn test_check_permissions_returns_status() {
    let status = permissions::check_all();
    // Should return valid booleans (either true or false, just not crash)
    println!(
        "Microphone: {}, Accessibility: {}",
        status.microphone, status.accessibility
    );
}

#[test]
fn test_permission_status_serializes() {
    let status = PermissionStatus {
        microphone: true,
        accessibility: false,
        paste_tools: None,
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("\"microphone\":true"));
    assert!(json.contains("\"accessibility\":false"));
    // paste_tools is None → skip_serializing_if skips it
    assert!(!json.contains("paste_tools"));
}
