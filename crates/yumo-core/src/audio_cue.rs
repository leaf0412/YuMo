//! 录音开始 / 结束提示音。
//!
//! 设计:
//! - `resolve_cue_source` 纯函数: 拿到 settings 决定该播什么 (禁用 / 默认音
//!   调 / 自定义文件)。可单测。
//! - `play_async` 副作用: 在后台线程拉起 rodio sink, 不阻塞调用方。播放
//!   失败静默忽略 — 提示音是辅助功能, 不该让录音管线因为播放器问题失败。
//!
//! UI 写 `sound_enabled` (bool) + `custom_sound_file` (路径字符串)。

use log::{debug, warn};
use rodio::source::{SineWave, Source};
use rodio::{Decoder, OutputStream, Sink};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueKind {
    Start,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CueSource {
    /// `sound_enabled=false` 或缺省 — 不播。
    Disabled,
    /// 用户给了 `custom_sound_file` (非空), 用 rodio 解码播放。
    CustomFile(PathBuf),
    /// 没有自定义文件 — 生成短促正弦音, 起 880Hz / 止 660Hz。
    DefaultTone {
        frequency_hz: f32,
        duration_ms: u64,
    },
}

const DEFAULT_DURATION_MS: u64 = 120;
const START_FREQ_HZ: f32 = 880.0;
const STOP_FREQ_HZ: f32 = 660.0;
const CUE_AMPLITUDE: f32 = 0.18;

/// 根据 settings + cue 类型决定该播什么。纯函数, 单测覆盖。
pub fn resolve_cue_source(settings: &HashMap<String, Value>, kind: CueKind) -> CueSource {
    let enabled = settings
        .get("sound_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enabled {
        return CueSource::Disabled;
    }
    if let Some(path) = settings
        .get("custom_sound_file")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return CueSource::CustomFile(PathBuf::from(path));
    }
    let frequency_hz = match kind {
        CueKind::Start => START_FREQ_HZ,
        CueKind::Stop => STOP_FREQ_HZ,
    };
    CueSource::DefaultTone {
        frequency_hz,
        duration_ms: DEFAULT_DURATION_MS,
    }
}

/// 异步播放一个 cue。失败静默吞 (warn 日志), 不阻塞调用方。
///
/// 不返回 JoinHandle — 提示音是 fire-and-forget, 调用方不需要等。
pub fn play_async(source: CueSource) {
    if matches!(source, CueSource::Disabled) {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(&source) {
            warn!("[audio_cue] play failed: {}", e);
        }
    });
}

fn play_blocking(source: &CueSource) -> Result<(), String> {
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| format!("open default output stream: {}", e))?;
    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| format!("create sink: {}", e))?;
    match source {
        CueSource::Disabled => return Ok(()),
        CueSource::CustomFile(path) => {
            let file = File::open(path).map_err(|e| format!("open {:?}: {}", path, e))?;
            let decoder = Decoder::new(BufReader::new(file))
                .map_err(|e| format!("decode {:?}: {}", path, e))?;
            sink.append(decoder);
        }
        CueSource::DefaultTone {
            frequency_hz,
            duration_ms,
        } => {
            let tone = SineWave::new(*frequency_hz)
                .take_duration(Duration::from_millis(*duration_ms))
                .amplify(CUE_AMPLITUDE);
            sink.append(tone);
        }
    }
    sink.sleep_until_end();
    debug!("[audio_cue] played {:?}", source);
    Ok(())
}
