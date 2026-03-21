use crate::error::AppResult;
use crate::platform::traits::PlatformPermissions;
use crate::platform::types::PermissionStatus;

pub struct LinuxPermissions;

impl PlatformPermissions for LinuxPermissions {
    fn check_microphone() -> bool {
        true // Linux doesn't have macOS-style explicit permission grants
    }

    fn check_accessibility() -> bool {
        true
    }

    fn check_all() -> PermissionStatus {
        PermissionStatus { microphone: true, accessibility: true }
    }

    fn request_microphone() -> AppResult<()> {
        Ok(())
    }

    fn open_microphone_settings() -> AppResult<()> {
        Ok(())
    }

    fn open_accessibility_settings() -> AppResult<()> {
        Ok(())
    }
}

pub fn check_microphone() -> bool {
    LinuxPermissions::check_microphone()
}

pub fn check_accessibility() -> bool {
    LinuxPermissions::check_accessibility()
}

pub fn check_all() -> PermissionStatus {
    LinuxPermissions::check_all()
}

pub fn request_microphone() {}

pub fn open_microphone_settings() {}

pub fn open_accessibility_settings() {}
