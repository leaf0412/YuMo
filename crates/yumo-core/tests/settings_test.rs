use serde_json::{json, Value};
use std::collections::HashMap;
use yumo_core::settings;

fn make(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

// ---------------------------------------------------------------------------
// resolve_paste_restore_delay_ms
// 语义: clipboard_restore=false 强制 0 (跳过 restore); 否则取 paste_delay 数值
//       (UI 键), 缺省 1500ms。修复历史 bug: UI 写 paste_delay, 后端读
//       clipboard_restore_delay → 永远拿不到用户的设置。
// ---------------------------------------------------------------------------

#[test]
fn restore_delay_default_when_no_settings() {
    let settings = make(&[]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 1500);
}

#[test]
fn restore_delay_reads_paste_delay_key() {
    let settings = make(&[("paste_delay", json!(800))]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 800);
}

#[test]
fn restore_delay_accepts_float() {
    // UI slider 可能给整数也可能给浮点; both must work
    let settings = make(&[("paste_delay", json!(250.0))]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 250);
}

#[test]
fn restore_delay_forced_zero_when_clipboard_restore_false() {
    // clipboard_restore=false → 0, paste_text 中 if restore_delay_ms > 0 跳过 restore
    let settings = make(&[
        ("paste_delay", json!(2000)),
        ("clipboard_restore", json!(false)),
    ]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 0);
}

#[test]
fn restore_delay_clipboard_restore_true_passes_through() {
    let settings = make(&[
        ("paste_delay", json!(500)),
        ("clipboard_restore", json!(true)),
    ]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 500);
}

#[test]
fn restore_delay_clipboard_restore_missing_defaults_to_enabled() {
    // 兼容老配置: 没写 clipboard_restore 时按 "开" 解释 (保持历史行为)
    let settings = make(&[("paste_delay", json!(500))]);
    assert_eq!(settings::resolve_paste_restore_delay_ms(&settings), 500);
}

// ---------------------------------------------------------------------------
// resolve_system_mute
// 语义: 读 UI 写的 system_mute 键 (历史代码读 system_mute_enabled 是死键)。
// ---------------------------------------------------------------------------

#[test]
fn system_mute_default_false() {
    let settings = make(&[]);
    assert!(!settings::resolve_system_mute(&settings));
}

#[test]
fn system_mute_reads_ui_key() {
    let settings = make(&[("system_mute", json!(true))]);
    assert!(settings::resolve_system_mute(&settings));
}

#[test]
fn system_mute_explicit_false() {
    let settings = make(&[("system_mute", json!(false))]);
    assert!(!settings::resolve_system_mute(&settings));
}

#[test]
fn system_mute_ignores_legacy_dead_key() {
    // system_mute_enabled 是历史死键; 不应再被识别 (避免老 DB 上的脏数据
    // 假装功能正常)
    let settings = make(&[("system_mute_enabled", json!(true))]);
    assert!(!settings::resolve_system_mute(&settings));
}
