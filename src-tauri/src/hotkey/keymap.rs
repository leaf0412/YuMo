/// Map browser KeyboardEvent.code to macOS CGKeyCode (Carbon HIToolbox).
pub fn browser_code_to_macos(code: &str) -> Option<u16> {
    Some(match code {
        // Letters
        "KeyA" => 0x00, "KeyS" => 0x01, "KeyD" => 0x02, "KeyF" => 0x03,
        "KeyH" => 0x04, "KeyG" => 0x05, "KeyZ" => 0x06, "KeyX" => 0x07,
        "KeyC" => 0x08, "KeyV" => 0x09, "KeyB" => 0x0B, "KeyQ" => 0x0C,
        "KeyW" => 0x0D, "KeyE" => 0x0E, "KeyR" => 0x0F, "KeyY" => 0x10,
        "KeyT" => 0x11, "KeyO" => 0x1F, "KeyU" => 0x20, "KeyI" => 0x22,
        "KeyP" => 0x23, "KeyL" => 0x25, "KeyJ" => 0x26, "KeyK" => 0x28,
        "KeyN" => 0x2D, "KeyM" => 0x2E,
        // Digits
        "Digit1" => 0x12, "Digit2" => 0x13, "Digit3" => 0x14, "Digit4" => 0x15,
        "Digit6" => 0x16, "Digit5" => 0x17, "Digit9" => 0x19, "Digit7" => 0x1A,
        "Digit8" => 0x1C, "Digit0" => 0x1D,
        // Modifiers (left/right distinguished)
        "ShiftLeft" => 0x38, "ShiftRight" => 0x3C,
        "ControlLeft" => 0x3B, "ControlRight" => 0x3E,
        "AltLeft" => 0x3A, "AltRight" => 0x3D,
        "MetaLeft" => 0x37, "MetaRight" => 0x36,
        "Fn" => 0x3F,
        // Function keys
        "F1" => 0x7A, "F2" => 0x78, "F3" => 0x63, "F4" => 0x76,
        "F5" => 0x60, "F6" => 0x61, "F7" => 0x62, "F8" => 0x64,
        "F9" => 0x65, "F10" => 0x6D, "F11" => 0x67, "F12" => 0x6F,
        // Special keys
        "Space" => 0x31, "Enter" => 0x24, "Tab" => 0x30,
        "Backspace" => 0x33, "Delete" => 0x75, "Escape" => 0x35,
        "ArrowUp" => 0x7E, "ArrowDown" => 0x7D, "ArrowLeft" => 0x7B, "ArrowRight" => 0x7C,
        "Home" => 0x73, "End" => 0x77, "PageUp" => 0x74, "PageDown" => 0x79,
        // Punctuation
        "Minus" => 0x1B, "Equal" => 0x18,
        "BracketLeft" => 0x21, "BracketRight" => 0x1E,
        "Backslash" => 0x2A, "Semicolon" => 0x29, "Quote" => 0x27,
        "Comma" => 0x2B, "Period" => 0x2F, "Slash" => 0x2C,
        "Backquote" => 0x32,
        _ => return None,
    })
}

/// 判断 browser code 是否为 modifier 键
pub fn is_modifier_code(code: &str) -> bool {
    matches!(
        code,
        "ShiftLeft" | "ShiftRight" | "ControlLeft" | "ControlRight"
            | "AltLeft" | "AltRight" | "MetaLeft" | "MetaRight" | "Fn"
    )
}
