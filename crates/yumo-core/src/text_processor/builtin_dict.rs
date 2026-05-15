//! Built-in homophone / typo dictionary, applied after the user's custom
//! Dictionary rules. Source data lives in `builtin_dictionary.json` and is
//! embedded at compile time via `include_str!`.
//!
//! Entries are classified at load time:
//! - Pure CJK source → plain substring replace (regex `\b` does not fire
//!   between CJK characters, so word-boundary matching would miss embedded
//!   occurrences like "用百渡搜索")
//! - Anything containing ASCII → case-insensitive word-boundary replace,
//!   same semantics as the user Dictionary

use log::{debug, info};
use regex::RegexBuilder;
use serde::Deserialize;
use std::sync::OnceLock;

const BUILTIN_DICT_JSON: &str = include_str!("builtin_dictionary.json");

#[derive(Deserialize)]
struct DictFile {
    #[allow(dead_code)]
    version: u32,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    from: String,
    to: String,
    #[serde(default)]
    #[allow(dead_code)]
    comment: String,
}

enum EntryKind {
    Cjk,
    Ascii,
}

struct Entry {
    from: String,
    to: String,
    kind: EntryKind,
}

fn is_cjk_char(c: char) -> bool {
    let u = c as u32;
    matches!(u,
        0x3040..=0x30FF      // hiragana / katakana
        | 0x3400..=0x4DBF    // CJK ext A
        | 0x4E00..=0x9FFF    // CJK unified
        | 0xF900..=0xFAFF    // CJK compatibility ideographs
    )
}

fn is_pure_cjk(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_cjk_char)
}

fn entries() -> &'static [Entry] {
    static CACHE: OnceLock<Vec<Entry>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let dict: DictFile = serde_json::from_str(BUILTIN_DICT_JSON)
            .expect("builtin_dictionary.json must be valid JSON");
        let count = dict.entries.len();
        let entries: Vec<Entry> = dict
            .entries
            .into_iter()
            .map(|raw| {
                let kind = if is_pure_cjk(&raw.from) {
                    EntryKind::Cjk
                } else {
                    EntryKind::Ascii
                };
                Entry {
                    from: raw.from,
                    to: raw.to,
                    kind,
                }
            })
            .collect();
        info!("[builtin_dict] loaded entries={}", count);
        entries
    })
}

/// Apply all built-in dictionary entries to `text`. Returns the modified
/// string. Entry order in the JSON file is preserved.
pub fn apply_builtin_dict(text: &str) -> String {
    let mut result = text.to_string();
    let mut applied = 0usize;
    for entry in entries() {
        match entry.kind {
            EntryKind::Cjk => {
                if result.contains(&entry.from) {
                    result = result.replace(&entry.from, &entry.to);
                    applied += 1;
                }
            }
            EntryKind::Ascii => {
                let pattern = format!(r"\b{}\b", regex::escape(&entry.from));
                if let Ok(re) = RegexBuilder::new(&pattern)
                    .case_insensitive(true)
                    .build()
                {
                    let new_result = re.replace_all(&result, entry.to.as_str()).to_string();
                    if new_result != result {
                        applied += 1;
                        result = new_result;
                    }
                }
            }
        }
    }
    debug!(
        "[builtin_dict] apply: total={} applied={}",
        entries().len(),
        applied
    );
    result
}
