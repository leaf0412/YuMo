//! Verify platform stub types are defined correctly (compile-time check)

#[cfg(target_os = "windows")]
mod windows_stubs {
    use yumo_core::platform::recorder;
    use yumo_core::platform::audio_ctrl;
    use yumo_core::platform::paster;
    use yumo_core::platform::permissions;
    use yumo_core::platform::keychain;

    #[test]
    fn windows_recorder_returns_err() {
        assert!(recorder::list_input_devices().is_err());
    }

    #[test]
    fn windows_audio_ctrl_returns_err() {
        assert!(audio_ctrl::is_system_muted().is_err());
    }

    #[test]
    fn windows_paster_stubs_are_noop() {
        assert!(paster::read_clipboard().is_none());
        assert!(paster::save_clipboard().is_none());
        paster::write_clipboard("test");
        paster::simulate_paste();
        paster::paste_text("test", 0);
    }

    #[test]
    fn windows_permissions_return_true() {
        assert!(permissions::check_microphone());
        assert!(permissions::check_accessibility());
        let status = permissions::check_all();
        assert!(status.microphone);
        assert!(status.accessibility);
    }

    #[test]
    fn windows_keychain_returns_err() {
        assert!(keychain::get_key("svc", "acct").is_err());
        assert!(keychain::store_key("svc", "acct", "pass").is_err());
        assert!(keychain::delete_key("svc", "acct").is_err());
    }
}

#[cfg(target_os = "linux")]
mod linux_stubs {
    use yumo_core::platform::recorder;
    use yumo_core::platform::audio_ctrl;
    use yumo_core::platform::paster;
    use yumo_core::platform::permissions;
    use yumo_core::platform::keychain;

    #[test]
    fn linux_recorder_returns_err() {
        assert!(recorder::list_input_devices().is_err());
    }

    #[test]
    fn linux_audio_ctrl_returns_err() {
        assert!(audio_ctrl::is_system_muted().is_err());
    }

    #[test]
    fn linux_paster_stubs_are_noop() {
        assert!(paster::read_clipboard().is_none());
        assert!(paster::save_clipboard().is_none());
        paster::write_clipboard("test");
        paster::simulate_paste();
        paster::paste_text("test", 0);
    }

    #[test]
    fn linux_permissions_return_true() {
        assert!(permissions::check_microphone());
        assert!(permissions::check_accessibility());
        let status = permissions::check_all();
        assert!(status.microphone);
        assert!(status.accessibility);
    }

    #[test]
    fn linux_keychain_returns_err() {
        assert!(keychain::get_key("svc", "acct").is_err());
        assert!(keychain::store_key("svc", "acct", "pass").is_err());
        assert!(keychain::delete_key("svc", "acct").is_err());
    }
}

// Cross-platform test: traits are always available regardless of OS
#[test]
fn test_traits_available_on_all_platforms() {
    use yumo_core::platform::traits::*;
    fn _r<T: PlatformRecorder>() {}
    fn _a<T: PlatformAudioCtrl>() {}
    fn _p<T: PlatformPaster>() {}
    fn _pm<T: PlatformPermissions>() {}
    fn _k<T: PlatformKeychain>() {}
}
