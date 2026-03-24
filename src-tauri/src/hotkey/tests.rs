use super::keymap;

#[test]
fn test_modifier_keys_mapped() {
    assert_eq!(keymap::browser_code_to_macos("ShiftRight"), Some(0x3C));
    assert_eq!(keymap::browser_code_to_macos("ShiftLeft"), Some(0x38));
    assert_eq!(keymap::browser_code_to_macos("MetaRight"), Some(0x36));
    assert_eq!(keymap::browser_code_to_macos("MetaLeft"), Some(0x37));
    assert_eq!(keymap::browser_code_to_macos("AltRight"), Some(0x3D));
    assert_eq!(keymap::browser_code_to_macos("AltLeft"), Some(0x3A));
    assert_eq!(keymap::browser_code_to_macos("ControlLeft"), Some(0x3B));
    assert_eq!(keymap::browser_code_to_macos("ControlRight"), Some(0x3E));
    assert_eq!(keymap::browser_code_to_macos("Fn"), Some(0x3F));
}

#[test]
fn test_common_keys_mapped() {
    assert_eq!(keymap::browser_code_to_macos("Space"), Some(0x31));
    assert_eq!(keymap::browser_code_to_macos("Escape"), Some(0x35));
    assert_eq!(keymap::browser_code_to_macos("KeyA"), Some(0x00));
    assert_eq!(keymap::browser_code_to_macos("F1"), Some(0x7A));
}

#[test]
fn test_unknown_code_returns_none() {
    assert_eq!(keymap::browser_code_to_macos("UnknownKey"), None);
}

#[test]
fn test_is_modifier_code() {
    assert!(keymap::is_modifier_code("ShiftRight"));
    assert!(keymap::is_modifier_code("MetaLeft"));
    assert!(keymap::is_modifier_code("Fn"));
    assert!(!keymap::is_modifier_code("KeyA"));
    assert!(!keymap::is_modifier_code("Space"));
    assert!(!keymap::is_modifier_code("F1"));
}
