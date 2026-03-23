use cocoa::foundation::NSString as NSStringTrait;
use objc::{msg_send, sel, sel_impl};

use crate::error::AppResult;
use crate::platform::traits::PlatformPermissions;
use crate::platform::types::PermissionStatus;

// ---------------------------------------------------------------------------
// MacosPermissions — PlatformPermissions implementation
// ---------------------------------------------------------------------------

pub struct MacosPermissions;

impl PlatformPermissions for MacosPermissions {
    fn check_microphone() -> bool {
        check_microphone_impl()
    }

    fn check_accessibility() -> bool {
        check_accessibility_impl()
    }

    fn check_all() -> PermissionStatus {
        PermissionStatus {
            microphone: check_microphone_impl(),
            accessibility: check_accessibility_impl(),
            paste_tools: None,
        }
    }

    fn request_microphone() -> AppResult<()> {
        log::info!("[permissions] [request] opening microphone settings");
        open_settings("Privacy_Microphone");
        Ok(())
    }

    fn open_microphone_settings() -> AppResult<()> {
        log::info!("[permissions] [request] opening microphone settings");
        open_settings("Privacy_Microphone");
        Ok(())
    }

    fn open_accessibility_settings() -> AppResult<()> {
        log::info!("[permissions] [request] opening accessibility settings");
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn check_microphone() -> bool {
    MacosPermissions::check_microphone()
}

pub fn check_accessibility() -> bool {
    MacosPermissions::check_accessibility()
}

pub fn check_all() -> PermissionStatus {
    MacosPermissions::check_all()
}

pub fn request_microphone() {
    let _ = MacosPermissions::request_microphone();
}

pub fn open_microphone_settings() {
    let _ = MacosPermissions::open_microphone_settings();
}

pub fn open_accessibility_settings() {
    let _ = MacosPermissions::open_accessibility_settings();
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

/// Check microphone permission using AVCaptureDevice authorization status.
/// Returns the raw status: 0=notDetermined, 1=restricted, 2=denied, 3=authorized.
fn microphone_status() -> i64 {
    unsafe {
        let cls = match objc::runtime::Class::get("AVCaptureDevice") {
            Some(cls) => cls,
            None => return 0,
        };
        let media_type = NSStringTrait::alloc(cocoa::base::nil)
            .init_str("soun"); // AVMediaTypeAudio = "soun"
        msg_send![cls, authorizationStatusForMediaType: media_type]
    }
}

fn check_microphone_impl() -> bool {
    let status = microphone_status();
    let devices = super::recorder::list_input_devices().unwrap_or_default();
    let result = if status == 3 {
        true
    } else if status == 2 {
        false
    } else {
        !devices.is_empty()
    };
    log::info!("[permissions] microphone: AVStatus={} devices={} result={}", status, devices.len(), result);
    result
}

fn check_accessibility_impl() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    let trusted = unsafe { AXIsProcessTrusted() };
    log::info!("[permissions] AXIsProcessTrusted = {}", trusted);
    trusted
}

/// Open a specific System Settings privacy pane.
fn open_settings(pane: &str) {
    let _ = std::process::Command::new("open")
        .arg(format!(
            "x-apple.systempreferences:com.apple.preference.security?{pane}"
        ))
        .spawn();
}
