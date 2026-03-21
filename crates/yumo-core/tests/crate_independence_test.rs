//! Verifies yumo-core can be used as a standalone crate
//! without any Tauri dependency.

use yumo_core::error::{AppError, AppResult};
use yumo_core::mask;
use yumo_core::pipeline::{self, Action, PipelineState};
use yumo_core::state::AppPaths;
use yumo_core::text_processor;

#[test]
fn test_core_types_standalone() {
    // AppPaths can be created without Tauri
    let paths = AppPaths::defaults();
    assert!(paths.data_dir.to_string_lossy().contains(".voiceink"));
    assert!(paths.models_dir.to_string_lossy().contains("models"));

    // Pipeline state machine works standalone
    let state = PipelineState::Idle;
    let next = pipeline::transition(state, Action::StartRecording);
    assert!(matches!(next, PipelineState::Recording));

    // Error types constructable
    let err: AppError = AppError::InvalidInput("test".into());
    assert!(err.to_string().contains("test"));

    // Mask function works
    let masked = mask::mask("secret_api_key_12345");
    assert_ne!(masked, "secret_api_key_12345");

    // Text processor works
    let result = text_processor::capitalize_sentences("hello world. foo bar.");
    assert!(result.starts_with("Hello"));
}

#[test]
fn test_pipeline_state_transitions() {
    // Full pipeline cycle: Idle -> Recording -> Processing -> Transcribing -> Pasting -> Idle
    let s = PipelineState::Idle;
    let s = pipeline::transition(s, Action::StartRecording);
    assert_eq!(s, PipelineState::Recording);

    let s = pipeline::transition(s, Action::StopRecording);
    assert_eq!(s, PipelineState::Processing);

    let s = pipeline::transition(s, Action::ProcessingComplete);
    assert_eq!(s, PipelineState::Transcribing);

    let s = pipeline::transition(s, Action::TranscriptionComplete);
    assert_eq!(s, PipelineState::Pasting);

    let s = pipeline::transition(s, Action::PasteComplete);
    assert_eq!(s, PipelineState::Idle);
}

#[test]
fn test_pipeline_cancel_from_any_state() {
    for state in [
        PipelineState::Recording,
        PipelineState::Processing,
        PipelineState::Transcribing,
        PipelineState::Enhancing,
        PipelineState::Pasting,
    ] {
        let next = pipeline::transition(state, Action::Cancel);
        assert_eq!(next, PipelineState::Idle, "Cancel from {:?} should go to Idle", state);
    }
}

#[test]
fn test_audio_input_device_struct() {
    use yumo_core::platform::{AudioData, AudioInputDevice, AudioLevel};

    let device = AudioInputDevice {
        id: 1,
        name: "Built-in Microphone".to_string(),
        is_default: true,
    };
    assert_eq!(device.id, 1);
    assert!(device.is_default);

    let level = AudioLevel { rms: 0.5, peak: 0.9 };
    assert!(level.rms < level.peak);

    let data = AudioData {
        pcm_samples: vec![0.0; 16000],
        sample_rate: 16000,
        channels: 1,
    };
    assert_eq!(data.pcm_samples.len(), 16000);
}

#[test]
fn test_mask_function_variants() {
    // Very short -> "***"
    assert_eq!(mask::mask("abc"), "***");
    // Medium (<=10) -> first 2 + last 2
    let m = mask::mask("abcdefgh");
    assert_eq!(m, "ab...gh");
    // Long (>10) -> first 4 + last 4
    let m = mask::mask("abcdefghijklmnop");
    assert_eq!(m, "abcd...mnop");
}

#[test]
fn test_app_result_type() {
    fn returns_ok() -> AppResult<i32> {
        Ok(42)
    }
    fn returns_err() -> AppResult<i32> {
        Err(AppError::InvalidInput("bad".into()))
    }

    assert_eq!(returns_ok().unwrap(), 42);
    assert!(returns_err().is_err());
}

#[test]
fn test_error_variants() {
    let variants: Vec<AppError> = vec![
        AppError::Database("db err".into()),
        AppError::Recording("rec err".into()),
        AppError::Transcription("tx err".into()),
        AppError::Network("net err".into()),
        AppError::Io("io err".into()),
        AppError::Permission("perm err".into()),
        AppError::NotFound("not found".into()),
        AppError::InvalidInput("invalid".into()),
    ];
    for err in &variants {
        // All variants should produce non-empty Display output
        assert!(!err.to_string().is_empty());
    }
}

#[test]
fn test_daemon_event_callback_type() {
    use yumo_core::daemon::DaemonEventCallback;

    // Verify DaemonEventCallback is constructable
    let callback: DaemonEventCallback = Box::new(|_name, _payload| {});
    callback("test-event", &serde_json::json!({"key": "value"}));
}

#[test]
fn test_text_processor_capitalize_sentences() {
    assert_eq!(
        text_processor::capitalize_sentences("hello. world. test"),
        "Hello. World. Test"
    );
    assert_eq!(text_processor::capitalize_sentences(""), "");
    assert_eq!(
        text_processor::capitalize_sentences("already Capital"),
        "Already Capital"
    );
}
