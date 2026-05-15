mod builtin_dict;
mod cn_numerals;
pub use builtin_dict::apply_builtin_dict;
pub use cn_numerals::convert_cn_numerals;

use crate::mask;
use log::{info, warn};
use regex::{Regex, RegexBuilder};
use std::sync::OnceLock;

/// Apply word-boundary-aware, case-insensitive replacements.
pub fn apply_replacements(text: &str, replacements: &[(String, String)]) -> String {
    info!("[text_processor] apply_replacements text_length={} replacements_count={}", text.len(), replacements.len());
    let mut result = text.to_string();
    let mut applied_count = 0;
    for (original, replacement) in replacements {
        let pattern = format!(r"\b{}\b", regex::escape(original));
        if let Ok(re) = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
        {
            let prev = result.clone();
            result = re.replace_all(&result, replacement.as_str()).to_string();
            if result != prev {
                applied_count += 1;
            }
        }
    }
    info!("[text_processor] [apply_replacements] input_len={} rules={} applied={}", text.len(), replacements.len(), applied_count);
    result
}

/// Capitalize the first letter after `.`, `?`, `!`, and at start of string.
pub fn capitalize_sentences(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
            if ch == '.' || ch == '?' || ch == '!' {
                capitalize_next = true;
            } else if !ch.is_whitespace() {
                capitalize_next = false;
            }
        }
    }

    info!("[text_processor] [capitalize] input_len={} output_len={}", text.len(), result.len());
    result
}

/// Detect and filter hallucinated transcription output from Whisper-like models.
///
/// Short audio clips often cause models to hallucinate: outputting parenthesized
/// text like "(字幕)", random symbols, or foreign-language fragments.
/// Returns `true` if the text looks like hallucination and should be discarded.
pub fn is_hallucinated(text: &str) -> bool {
    let trimmed = text.trim();

    // Empty or whitespace-only
    if trimmed.is_empty() {
        return true;
    }

    // Entire output is wrapped in parentheses / brackets — e.g. "(字幕由...)", "[音乐]"
    if (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('（') && trimmed.ends_with('）'))
        || (trimmed.starts_with('【') && trimmed.ends_with('】'))
    {
        warn!("[text_processor] hallucination detected (bracketed): {}", mask::mask_text(trimmed));
        return true;
    }

    // Mostly non-alphanumeric symbols (allow CJK as "alphanumeric")
    let meaningful: usize = trimmed
        .chars()
        .filter(|c| c.is_alphanumeric() || *c >= '\u{4e00}' && *c <= '\u{9fff}' || *c >= '\u{3040}' && *c <= '\u{30ff}')
        .count();
    let total: usize = trimmed.chars().filter(|c| !c.is_whitespace()).count();
    if total > 0 && (meaningful as f64 / total as f64) < 0.3 {
        warn!("[text_processor] hallucination detected (symbols): {}", mask::mask_text(trimmed));
        return true;
    }

    // Repeated single token — e.g. "ん ん ん ん" or "... ... ..."
    if let Ok(re) = Regex::new(r"^(.{1,4})\s*(\1\s*){2,}$") {
        if re.is_match(trimmed) {
            warn!("[text_processor] hallucination detected (repeated): {}", mask::mask_text(trimmed));
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Merge whitespace-separated uppercase letters back into acronyms.
// ---------------------------------------------------------------------------

fn uppercase_letter_seq_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b[A-Z](?:[ \t]+[A-Z]\b)+").unwrap())
}

/// Collapse sequences of isolated uppercase letters (≥2 letters separated by
/// whitespace) into a single acronym token. Whisper-class models often emit
/// acronyms like CDN / URL / API / MR as "C D N", but the speaker's intent is
/// the acronym — so we merge them back.
///
/// Lowercase letters or other tokens break the sequence, so "A the B" stays
/// unchanged. Single uppercase letters (e.g. "维生素 A") are also untouched.
pub fn merge_uppercase_letter_sequences(text: &str) -> String {
    uppercase_letter_seq_re()
        .replace_all(text, |caps: &regex::Captures| {
            caps[0]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        })
        .into_owned()
}

// ---------------------------------------------------------------------------
// CJK ↔ ASCII spacing (PangU style)
// ---------------------------------------------------------------------------

fn cjk_then_ascii_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"([\u{3040}-\u{30ff}\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}])([A-Za-z0-9])").unwrap()
    })
}

fn ascii_then_cjk_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"([A-Za-z0-9])([\u{3040}-\u{30ff}\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}])").unwrap()
    })
}

/// Insert a single space at every CJK ↔ ASCII letter/digit boundary.
/// Idempotent: existing spaces or punctuation between CJK/ASCII are preserved.
pub fn add_cjk_spacing(text: &str) -> String {
    let s = cjk_then_ascii_re().replace_all(text, "$1 $2");
    ascii_then_cjk_re().replace_all(&s, "$1 $2").into_owned()
}

// ---------------------------------------------------------------------------
// Append terminal period (locale-adaptive)
// ---------------------------------------------------------------------------

/// Append a sentence-ending period to `text` when it does not already end with
/// one. Picks `。` for CJK-ending text, `.` for ASCII-ending text. Returns the
/// original text unchanged when empty / whitespace-only or already terminated
/// by `.?!。？！…⋯`.
pub fn append_terminal_period(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return text.to_string();
    }
    let last = match trimmed.chars().last() {
        Some(c) => c,
        None => return text.to_string(),
    };
    if matches!(last, '.' | '?' | '!' | '。' | '？' | '！' | '…' | '⋯') {
        return text.to_string();
    }
    let suffix = if is_cjk_terminator_char(last) { "。" } else { "." };
    let mut s = String::with_capacity(trimmed.len() + suffix.len());
    s.push_str(trimmed);
    s.push_str(suffix);
    s
}

fn is_cjk_terminator_char(c: char) -> bool {
    let u = c as u32;
    matches!(u,
        0x3040..=0x30FF      // hiragana / katakana
        | 0x3400..=0x4DBF    // CJK ext A
        | 0x4E00..=0x9FFF    // CJK unified
        | 0xF900..=0xFAFF    // CJK compatibility ideographs
        | 0xFF00..=0xFFEF    // halfwidth / fullwidth forms
        | 0x3000..=0x303F    // CJK symbols & punctuation
    )
}

// ---------------------------------------------------------------------------
// process_text — pipeline integration
// ---------------------------------------------------------------------------

/// Bundle of post-processing toggles. Each field maps 1:1 to a UI Switch in
/// Settings (see `_docs/specs/2026-05-15-transcript-postprocess-toggles-design.md`).
///
/// `Default::default()` returns all-false. Product defaults are decided by
/// the settings read site (`unwrap_or(...)` in `src-tauri/src/commands.rs`),
/// **not** by this `Default` impl — keep these separate so tests can build a
/// clean baseline without inheriting product defaults.
#[derive(Debug, Clone, Default)]
pub struct ProcessOptions {
    pub auto_capitalize: bool,
    pub append_period: bool,
    pub convert_cn_numerals: bool,
    pub use_builtin_dictionary: bool,
}

/// Apply the full transcript post-processing pipeline. The four conditional
/// steps are gated by `opts`; the unconditional steps (user replacements,
/// uppercase-letter merge, CJK/ASCII spacing) always run.
pub fn process_text(
    text: &str,
    user_replacements: &[(String, String)],
    opts: &ProcessOptions,
) -> String {
    info!(
        "[text_processor] process_text input={} opts={:?}",
        mask::mask_text(text),
        opts
    );
    let s = apply_replacements(text, user_replacements);
    let s = if opts.use_builtin_dictionary {
        builtin_dict::apply_builtin_dict(&s)
    } else {
        s
    };
    let s = merge_uppercase_letter_sequences(&s);
    let s = if opts.convert_cn_numerals {
        convert_cn_numerals(&s)
    } else {
        s
    };
    let s = add_cjk_spacing(&s);
    let s = if opts.auto_capitalize {
        capitalize_sentences(&s)
    } else {
        s
    };
    let s = if opts.append_period {
        append_terminal_period(&s)
    } else {
        s
    };
    info!("[text_processor] process_text output={}", mask::mask_text(&s));
    s
}
