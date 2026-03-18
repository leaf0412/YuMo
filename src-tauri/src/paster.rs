use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::runtime::Class;
use objc::{msg_send, sel, sel_impl};

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

/// Read the current string contents of the system clipboard.
pub fn read_clipboard() -> Option<String> {
    unsafe {
        let pasteboard = general_pasteboard();
        let pb_type = pasteboard_type_string();
        let string: id = msg_send![pasteboard, stringForType: pb_type];
        if string == nil {
            return None;
        }
        let c_str: *const std::os::raw::c_char = msg_send![string, UTF8String];
        if c_str.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr(c_str)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// Write a string to the system clipboard, replacing any existing content.
pub fn write_clipboard(text: &str) {
    unsafe {
        let pasteboard = general_pasteboard();
        let pb_type = pasteboard_type_string();
        let ns_string = NSString::alloc(nil).init_str(text);

        // Build NSArray with one element: the pasteboard type
        let cls = Class::get("NSArray").expect("NSArray class not found");
        let types: id = msg_send![cls, arrayWithObject: pb_type];

        // declareTypes:owner: clears and declares
        let _: isize = msg_send![pasteboard, declareTypes: types owner: nil];
        let _: bool =
            msg_send![pasteboard, setString: ns_string forType: pb_type];
    }
}

/// Snapshot the current clipboard contents for later restoration.
pub fn save_clipboard() -> Option<String> {
    read_clipboard()
}

/// Restore previously saved clipboard contents. If `saved` is None, the
/// clipboard is left unchanged.
pub fn restore_clipboard(saved: Option<String>) {
    if let Some(text) = saved {
        write_clipboard(&text);
    }
}

/// Simulate a Cmd+V keystroke via CGEvent.
///
/// This requires the application to have Accessibility permission
/// (System Settings > Privacy & Security > Accessibility).
pub fn simulate_paste() {
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

/// Full paste flow: save clipboard, write text, simulate Cmd+V, then
/// asynchronously restore the original clipboard after `restore_delay_ms`.
pub fn paste_text(text: &str, restore_delay_ms: u64) {
    let saved = save_clipboard();
    write_clipboard(text);
    simulate_paste();

    if restore_delay_ms > 0 {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(
                restore_delay_ms,
            ));
            restore_clipboard(saved);
        });
    }
}
