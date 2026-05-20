use serde_json::{json, Value};
use std::collections::HashMap;
use yumo_core::audio_cue::{resolve_cue_source, CueKind, CueSource};

fn make(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

#[test]
fn cue_disabled_when_sound_enabled_false() {
    let s = make(&[("sound_enabled", json!(false))]);
    assert!(matches!(resolve_cue_source(&s, CueKind::Start), CueSource::Disabled));
    assert!(matches!(resolve_cue_source(&s, CueKind::Stop), CueSource::Disabled));
}

#[test]
fn cue_disabled_by_default_when_setting_missing() {
    // sound_enabled 未写入 DB 时默认关 — 保守默认, 不打扰用户
    let s = make(&[]);
    assert!(matches!(resolve_cue_source(&s, CueKind::Start), CueSource::Disabled));
}

#[test]
fn cue_default_tone_when_enabled_no_custom_file() {
    let s = make(&[("sound_enabled", json!(true))]);
    match resolve_cue_source(&s, CueKind::Start) {
        CueSource::DefaultTone { frequency_hz, .. } => {
            assert!(frequency_hz > 0.0, "start tone freq must be positive");
        }
        other => panic!("expected DefaultTone, got {:?}", other),
    }
}

#[test]
fn cue_start_and_stop_use_different_frequencies() {
    // 起 / 止 听感要不同, 否则用户分不清当前是开始还是结束
    let s = make(&[("sound_enabled", json!(true))]);
    let start = match resolve_cue_source(&s, CueKind::Start) {
        CueSource::DefaultTone { frequency_hz, .. } => frequency_hz,
        _ => unreachable!(),
    };
    let stop = match resolve_cue_source(&s, CueKind::Stop) {
        CueSource::DefaultTone { frequency_hz, .. } => frequency_hz,
        _ => unreachable!(),
    };
    assert_ne!(start, stop, "start / stop frequencies must differ");
}

#[test]
fn cue_custom_file_when_enabled_and_path_set() {
    let s = make(&[
        ("sound_enabled", json!(true)),
        ("custom_sound_file", json!("/tmp/my-beep.wav")),
    ]);
    match resolve_cue_source(&s, CueKind::Start) {
        CueSource::CustomFile(p) => {
            assert_eq!(p.to_string_lossy(), "/tmp/my-beep.wav");
        }
        other => panic!("expected CustomFile, got {:?}", other),
    }
}

#[test]
fn cue_empty_custom_file_falls_back_to_default_tone() {
    // UI 里清空输入框 = "" 是常见情况, 不应当作有效路径去 open(""), 直接走默认
    let s = make(&[
        ("sound_enabled", json!(true)),
        ("custom_sound_file", json!("")),
    ]);
    assert!(matches!(
        resolve_cue_source(&s, CueKind::Start),
        CueSource::DefaultTone { .. }
    ));
}

#[test]
fn cue_whitespace_custom_file_falls_back_to_default() {
    let s = make(&[
        ("sound_enabled", json!(true)),
        ("custom_sound_file", json!("   ")),
    ]);
    assert!(matches!(
        resolve_cue_source(&s, CueKind::Start),
        CueSource::DefaultTone { .. }
    ));
}
