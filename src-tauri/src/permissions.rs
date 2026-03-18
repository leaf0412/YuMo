use cocoa::foundation::NSString as NSStringTrait;
use objc::{msg_send, sel, sel_impl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub microphone: bool,
    pub accessibility: bool,
}

/// Check microphone permission using AVCaptureDevice authorization status.
pub fn check_microphone() -> bool {
    unsafe {
        let cls = match objc::runtime::Class::get("AVCaptureDevice") {
            Some(cls) => cls,
            None => return false,
        };
        let media_type = NSStringTrait::alloc(cocoa::base::nil)
            .init_str("soun"); // AVMediaTypeAudio = "soun"
        let status: i64 = objc::msg_send![cls, authorizationStatusForMediaType: media_type];
        status == 3 // AVAuthorizationStatusAuthorized
    }
}

/// Check accessibility permission using AXIsProcessTrusted.
pub fn check_accessibility() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
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

/// Open System Settings to the microphone pane.
pub fn open_microphone_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn();
}
