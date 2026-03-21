use yumo_core::permissions;

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
    let status = permissions::PermissionStatus {
        microphone: true,
        accessibility: false,
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("\"microphone\":true"));
    assert!(json.contains("\"accessibility\":false"));
}
