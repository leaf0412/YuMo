//! Verifies platform trait abstraction works end-to-end.

use yumo_core::platform::traits::*;
use yumo_core::platform::{AudioInputDevice, AudioData, AudioLevel, PermissionStatus};

#[test]
fn test_traits_importable() {
    fn _assert_recorder<T: PlatformRecorder>() {}
    fn _assert_audio_ctrl<T: PlatformAudioCtrl>() {}
    fn _assert_paster<T: PlatformPaster>() {}
    fn _assert_permissions<T: PlatformPermissions>() {}
    fn _assert_keychain<T: PlatformKeychain>() {}
}

#[test]
fn test_types_constructable_without_cfg() {
    let device = AudioInputDevice { id: 1, name: "Test".into(), is_default: true };
    let level = AudioLevel { rms: 0.5, peak: 0.8 };
    let data = AudioData { pcm_samples: vec![0.0; 100], sample_rate: 16000, channels: 1 };
    let status = PermissionStatus { microphone: true, accessibility: false };
    assert_eq!(device.id, 1);
    assert!(level.rms < level.peak);
    assert_eq!(data.sample_rate, 16000);
    assert!(status.microphone);
}

#[cfg(target_os = "macos")]
mod macos_integration {
    use yumo_core::platform::recorder;
    use yumo_core::platform::audio_ctrl;
    use yumo_core::platform::permissions;
    use yumo_core::audio_io;
    use yumo_core::platform::AudioData;
    use yumo_core::pipeline::{PipelineState, Action, transition};

    #[test]
    fn test_pipeline_state_transitions() {
        let state = PipelineState::Idle;
        let state = transition(state, Action::StartRecording);
        assert!(matches!(state, PipelineState::Recording));
    }

    #[test]
    fn test_device_listing() {
        let result = recorder::list_input_devices();
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_io_with_platform_types() {
        let data = AudioData {
            pcm_samples: vec![0.0f32; 16000],
            sample_rate: 16000,
            channels: 1,
        };
        let path = std::env::temp_dir().join("test_platform_integration.wav");
        audio_io::save_wav(&data, &path).unwrap();
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_permissions_via_trait() {
        let status = permissions::check_all();
        let _ = status.microphone;
        let _ = status.accessibility;
    }

    #[test]
    fn test_audio_ctrl_accessible() {
        let result = audio_ctrl::is_system_muted();
        assert!(result.is_ok());
    }
}
