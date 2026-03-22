//! Cross-platform integration tests for Windows / Linux implementations.
//!
//! - recorder / audio_ctrl: 真实实现（cpal / pactl），CI 可能无音频设备，
//!   只验证不 panic，不断言 Ok/Err。
//! - paster: 需要窗口环境，标记 #[ignore]。
//! - keychain: 需要系统密钥服务（Credential Manager / D-Bus secret-service），标记 #[ignore]。
//! - permissions: 当前返回 true（无需系统权限），可正常跑。

#[cfg(target_os = "windows")]
mod windows_platform {
    use yumo_core::platform::audio_ctrl;
    use yumo_core::platform::keychain;
    use yumo_core::platform::paster;
    use yumo_core::platform::permissions;
    use yumo_core::platform::recorder;

    #[test]
    fn recorder_does_not_panic() {
        let _ = recorder::list_input_devices();
    }

    #[test]
    fn audio_ctrl_does_not_panic() {
        let _ = audio_ctrl::is_system_muted();
    }

    #[test]
    #[ignore] // 需要窗口环境
    fn paster_clipboard_roundtrip() {
        paster::write_clipboard("windows test");
        assert_eq!(
            paster::read_clipboard(),
            Some("windows test".to_string())
        );
    }

    #[test]
    fn permissions_return_true() {
        assert!(permissions::check_microphone());
        assert!(permissions::check_accessibility());
        let status = permissions::check_all();
        assert!(status.microphone);
        assert!(status.accessibility);
    }

    #[test]
    #[ignore] // 需要 Windows Credential Manager
    fn keychain_store_and_get() {
        keychain::store_key("yumo-test", "ci", "secret").unwrap();
        let val = keychain::get_key("yumo-test", "ci").unwrap();
        assert_eq!(val, Some("secret".to_string()));
        keychain::delete_key("yumo-test", "ci").unwrap();
    }
}

#[cfg(target_os = "linux")]
mod linux_platform {
    use yumo_core::platform::audio_ctrl;
    use yumo_core::platform::keychain;
    use yumo_core::platform::paster;
    use yumo_core::platform::permissions;
    use yumo_core::platform::recorder;

    #[test]
    fn recorder_does_not_panic() {
        let _ = recorder::list_input_devices();
    }

    #[test]
    fn audio_ctrl_does_not_panic() {
        let _ = audio_ctrl::is_system_muted();
    }

    #[test]
    #[ignore] // 需要窗口环境（X11/Wayland display）
    fn paster_clipboard_roundtrip() {
        paster::write_clipboard("linux test");
        assert_eq!(
            paster::read_clipboard(),
            Some("linux test".to_string())
        );
    }

    #[test]
    fn permissions_return_true() {
        assert!(permissions::check_microphone());
        assert!(permissions::check_accessibility());
        let status = permissions::check_all();
        assert!(status.microphone);
        assert!(status.accessibility);
    }

    #[test]
    #[ignore] // 需要 D-Bus secret-service
    fn keychain_store_and_get() {
        keychain::store_key("yumo-test", "ci", "secret").unwrap();
        let val = keychain::get_key("yumo-test", "ci").unwrap();
        assert_eq!(val, Some("secret".to_string()));
        keychain::delete_key("yumo-test", "ci").unwrap();
    }
}

// 跨平台：trait 定义在所有 OS 上可用
#[test]
fn traits_available_on_all_platforms() {
    use yumo_core::platform::traits::*;
    fn _r<T: PlatformRecorder>() {}
    fn _a<T: PlatformAudioCtrl>() {}
    fn _p<T: PlatformPaster>() {}
    fn _pm<T: PlatformPermissions>() {}
    fn _k<T: PlatformKeychain>() {}
}
