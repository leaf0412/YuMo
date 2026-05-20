//! 从 UI 持久化的 settings HashMap 解析出语义化的运行时值。
//!
//! 单独抽出来是为了:
//! 1) 集中 UI key ↔ 后端 key 的映射, 避免历史上 "UI 写一个名、后端读另一个
//!    名" 的对不齐 (paste_delay vs clipboard_restore_delay,
//!    system_mute vs system_mute_enabled)。
//! 2) 纯函数, 可以脱离 AppContext 单测。
//!
//! 任何新的 settings 派生逻辑都应加在这里, 而不是散在 commands.rs。

use serde_json::Value;
use std::collections::HashMap;

/// 默认 1500ms 恢复延迟 — 与历史行为对齐。 100ms 在某些 app 下 paste 还没完
/// 就 restore, 会把用户原剪贴板内容粘出去。
const DEFAULT_RESTORE_DELAY_MS: u64 = 1500;

/// 解析粘贴后剪贴板恢复延迟。返回 0 表示 paster 不应 restore (paster.rs 中
/// `if restore_delay_ms > 0` 的旁路)。
///
/// 规则:
/// - `clipboard_restore == false` → 强制 0
/// - 否则取 `paste_delay` 数值, 缺省 1500ms
///
/// UI 历史写的是 `paste_delay`, 后端却读 `clipboard_restore_delay`, 该键全
/// 工程没人写, 用户调整 slider 永远不生效。
pub fn resolve_paste_restore_delay_ms(settings: &HashMap<String, Value>) -> u64 {
    let restore_enabled = settings
        .get("clipboard_restore")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !restore_enabled {
        return 0;
    }
    settings
        .get("paste_delay")
        .and_then(|v| v.as_f64())
        .map(|v| v as u64)
        .unwrap_or(DEFAULT_RESTORE_DELAY_MS)
}

/// 录音时是否静音系统输出。
///
/// UI 写的是 `system_mute`, 但生产代码 (commands.rs:79/195/667) 一直读
/// `system_mute_enabled` — 一个全工程没人写的死键。结果 UI toggle 完全无效,
/// 这里统一只读 UI 键。
pub fn resolve_system_mute(settings: &HashMap<String, Value>) -> bool {
    settings
        .get("system_mute")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
