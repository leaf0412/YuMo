use std::sync::Mutex;
use yumo_core::platform::paster;

// Clipboard is shared mutable state — serialize all tests.
static CLIPBOARD_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[ignore] // 需要窗口环境（CI 无 display server）
fn test_clipboard_write_and_read() {
    let _guard = CLIPBOARD_LOCK.lock().unwrap();

    let original = paster::read_clipboard();

    paster::write_clipboard("voiceink paster test 98765");
    let result = paster::read_clipboard();
    assert_eq!(result, Some("voiceink paster test 98765".to_string()));

    // Restore original clipboard
    if let Some(orig) = original {
        paster::write_clipboard(&orig);
    }
}

#[test]
#[ignore] // 需要窗口环境（CI 无 display server）
fn test_save_and_restore_clipboard() {
    let _guard = CLIPBOARD_LOCK.lock().unwrap();

    let original = paster::read_clipboard();

    // Save current clipboard
    let saved = paster::save_clipboard();

    // Write something new
    paster::write_clipboard("temporary paster content");
    assert_eq!(
        paster::read_clipboard(),
        Some("temporary paster content".to_string())
    );

    // Restore saved content
    paster::restore_clipboard(saved);
    assert_eq!(paster::read_clipboard(), original);
}

#[test]
#[ignore] // 需要窗口环境（CI 无 display server）
fn test_write_empty_string() {
    let _guard = CLIPBOARD_LOCK.lock().unwrap();

    paster::write_clipboard("");
    let result = paster::read_clipboard();
    assert_eq!(result, Some("".to_string()));
}
