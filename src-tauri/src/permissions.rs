use cocoa::foundation::NSString as NSStringTrait;
use objc::{msg_send, sel, sel_impl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub microphone: bool,
    pub accessibility: bool,
}

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

pub fn check_microphone() -> bool {
    let status = microphone_status();
    let devices = crate::recorder::list_input_devices();
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

/// Open microphone settings.
pub fn request_microphone() {
    open_settings("Privacy_Microphone");
}

/// Check accessibility permission using AXIsProcessTrusted.
pub fn check_accessibility() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    let trusted = unsafe { AXIsProcessTrusted() };
    log::info!("[permissions] AXIsProcessTrusted = {}", trusted);
    trusted
}

/// Check all permissions at once.
pub fn check_all() -> PermissionStatus {
    PermissionStatus {
        microphone: check_microphone(),
        accessibility: check_accessibility(),
    }
}

/// Open System Settings to the accessibility pane.
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Open a specific System Settings privacy pane.
fn open_settings(pane: &str) {
    let _ = std::process::Command::new("open")
        .arg(format!(
            "x-apple.systempreferences:com.apple.preference.security?{pane}"
        ))
        .spawn();
}

/// Open System Settings to the microphone pane.
pub fn open_microphone_settings() {
    open_settings("Privacy_Microphone");
}
