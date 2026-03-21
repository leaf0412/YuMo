//! Tests DaemonEventCallback mechanism that replaced tauri::Emitter.

use std::sync::{Arc, Mutex};
use yumo_core::daemon::{DaemonEventCallback, DaemonManager};

#[test]
fn test_daemon_event_callback_receives_events() {
    // Simulate what src-tauri does: create a callback that captures events
    let captured_events: Arc<Mutex<Vec<(String, serde_json::Value)>>> =
        Arc::new(Mutex::new(Vec::new()));

    let events_clone = captured_events.clone();
    let callback: DaemonEventCallback = Box::new(move |event_name, payload| {
        events_clone
            .lock()
            .unwrap()
            .push((event_name.to_string(), payload.clone()));
    });

    // Invoke the callback as daemon.rs would
    callback(
        "daemon-setup-status",
        &serde_json::json!({"step": "checking_python", "progress": 0.1}),
    );
    callback(
        "daemon-status-changed",
        &serde_json::json!({"status": "running"}),
    );

    let events = captured_events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].0, "daemon-setup-status");
    assert_eq!(events[0].1["step"], "checking_python");
    assert_eq!(events[1].0, "daemon-status-changed");
    assert_eq!(events[1].1["status"], "running");
}

#[test]
fn test_daemon_event_callback_is_send_sync() {
    // Verify the callback type satisfies Send + Sync (required for cross-thread use)
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DaemonEventCallback>();
}

#[test]
fn test_daemon_manager_construction() {
    // DaemonManager should construct without panic
    let dm = DaemonManager::new(
        std::path::PathBuf::from("/tmp/test_daemon.py"),
        std::path::PathBuf::from("/tmp/test_data"),
    );
    // Verify initial state
    assert!(!dm.is_running());
    assert!(dm.loaded_model().is_none());
    drop(dm);
}

#[test]
fn test_daemon_manager_model_tracking() {
    let dm = DaemonManager::new(
        std::path::PathBuf::from("/tmp/test_daemon.py"),
        std::path::PathBuf::from("/tmp/test_data"),
    );

    // Initially no model loaded
    assert!(dm.loaded_model().is_none());

    // Set a model
    dm.set_loaded_model(Some("mlx-community/whisper-large-v3".to_string()));
    assert_eq!(
        dm.loaded_model(),
        Some("mlx-community/whisper-large-v3".to_string())
    );

    // Clear model
    dm.set_loaded_model(None);
    assert!(dm.loaded_model().is_none());
}

#[test]
fn test_daemon_response_deserialization() {
    use yumo_core::daemon::DaemonResponse;

    let json = r#"{"status":"ready"}"#;
    let resp: DaemonResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, "ready");
    assert!(resp.text.is_none());
    assert!(resp.error.is_none());

    let json = r#"{"status":"success","text":"hello world"}"#;
    let resp: DaemonResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, "success");
    assert_eq!(resp.text.as_deref(), Some("hello world"));
}

#[test]
fn test_callback_with_complex_payload() {
    let captured: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let captured_clone = captured.clone();
    let callback: DaemonEventCallback = Box::new(move |_name, payload| {
        captured_clone.lock().unwrap().push(payload.clone());
    });

    // Simulate a progress event with nested data
    let payload = serde_json::json!({
        "stage": "installing_deps",
        "progress": 0.75,
        "details": {
            "package": "mlx-audio-plus",
            "version": "0.1.0"
        }
    });
    callback("daemon-setup-status", &payload);

    let events = captured.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["stage"], "installing_deps");
    assert_eq!(events[0]["details"]["package"], "mlx-audio-plus");
}
