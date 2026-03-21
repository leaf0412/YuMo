use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use crate::error::AppResult;
use crate::mask;
use crate::platform::traits::PlatformPaster;
use log::{error, info};
use objc::runtime::Class;
use objc::{msg_send, sel, sel_impl};

// ---------------------------------------------------------------------------
// MacosPaster — PlatformPaster implementation
// ---------------------------------------------------------------------------

pub struct MacosPaster;

impl PlatformPaster for MacosPaster {
    fn read_clipboard() -> AppResult<Option<String>> {
        Ok(read_clipboard_impl())
    }

    fn write_clipboard(text: &str) -> AppResult<()> {
        write_clipboard_impl(text);
        Ok(())
    }

    fn save_clipboard() -> AppResult<Option<String>> {
        Ok(save_clipboard_impl())
    }

    fn restore_clipboard(saved: Option<String>) -> AppResult<()> {
        restore_clipboard_impl(saved);
        Ok(())
    }

    fn simulate_paste() -> AppResult<()> {
        simulate_paste_impl();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn read_clipboard() -> Option<String> {
    read_clipboard_impl()
}

pub fn write_clipboard(text: &str) {
    write_clipboard_impl(text);
}

pub fn save_clipboard() -> Option<String> {
    save_clipboard_impl()
}

pub fn restore_clipboard(saved: Option<String>) {
    restore_clipboard_impl(saved);
}

pub fn simulate_paste() {
    simulate_paste_impl();
}

/// Full paste flow: save clipboard, write text, simulate Cmd+V, then
/// asynchronously restore the original clipboard after `restore_delay_ms`.
pub fn paste_text(text: &str, restore_delay_ms: u64) {
    info!("[paster] paste_text start, text={} restore_delay_ms={}", mask::mask_text(text), restore_delay_ms);
    let saved = save_clipboard();
    write_clipboard(text);
    simulate_paste();

    if restore_delay_ms > 0 {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(restore_delay_ms));
            restore_clipboard(saved);
        });
    }
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

/// Get NSPasteboard.generalPasteboard via raw objc.
unsafe fn general_pasteboard() -> id {
    let cls = Class::get("NSPasteboard").expect("NSPasteboard class not found");
    msg_send![cls, generalPasteboard]
}

/// Get the NSString constant "public.utf8-plain-text" (NSPasteboardTypeString).
unsafe fn pasteboard_type_string() -> id {
    let ns = NSString::alloc(nil).init_str("public.utf8-plain-text");
    ns
}

fn read_clipboard_impl() -> Option<String> {
    info!("[paster] reading clipboard");
    unsafe {
        let pasteboard = general_pasteboard();
        let pb_type = pasteboard_type_string();
        let string: id = msg_send![pasteboard, stringForType: pb_type];
        if string == nil {
            info!("[paster] clipboard is empty");
            return None;
        }
        let c_str: *const std::os::raw::c_char = msg_send![string, UTF8String];
        if c_str.is_null() {
            error!("[paster] clipboard UTF8String returned null");
            return None;
        }
        let result = std::ffi::CStr::from_ptr(c_str)
            .to_string_lossy()
            .into_owned();
        info!("[paster] clipboard read ok, length={}", result.len());
        Some(result)
    }
}

fn write_clipboard_impl(text: &str) {
    info!("[paster] writing to clipboard, text={}", mask::mask_text(text));
    unsafe {
        let pasteboard = general_pasteboard();
        let pb_type = pasteboard_type_string();
        let ns_string = NSString::alloc(nil).init_str(text);

        // Build NSArray with one element: the pasteboard type
        let cls = Class::get("NSArray").expect("NSArray class not found");
        let types: id = msg_send![cls, arrayWithObject: pb_type];

        // declareTypes:owner: clears and declares
        let _: isize = msg_send![pasteboard, declareTypes: types owner: nil];
        let _: bool = msg_send![pasteboard, setString: ns_string forType: pb_type];
    }
}

fn save_clipboard_impl() -> Option<String> {
    info!("[paster] saving clipboard snapshot");
    read_clipboard_impl()
}

fn restore_clipboard_impl(saved: Option<String>) {
    info!("[paster] restoring clipboard, has_saved={}", saved.is_some());
    if let Some(text) = saved {
        write_clipboard_impl(&text);
    }
}

fn simulate_paste_impl() {
    info!("[paster] simulating Cmd+V paste");
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create CGEventSource");

    // Virtual key codes: 0x09 = V
    let v_down = CGEvent::new_keyboard_event(source.clone(), 0x09, true)
        .expect("Failed to create key-down event");
    let v_up = CGEvent::new_keyboard_event(source.clone(), 0x09, false)
        .expect("Failed to create key-up event");

    v_down.set_flags(CGEventFlags::CGEventFlagCommand);
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);

    v_down.post(CGEventTapLocation::HID);
    v_up.post(CGEventTapLocation::HID);
}
