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
// Chinese numerals → Arabic digits
// ---------------------------------------------------------------------------

fn cn_digit_value(c: char) -> Option<i64> {
    match c {
        '〇' | '零' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn cn_unit_value(c: char) -> Option<i64> {
    match c {
        '十' => Some(10),
        '百' => Some(100),
        '千' => Some(1000),
        '万' => Some(10_000),
        '亿' => Some(100_000_000),
        _ => None,
    }
}

/// Parse a token consisting only of CJK numeral characters into i64.
/// Returns `None` for malformed/ambiguous tokens (caller should leave original).
fn parse_cn_numeral(s: &str) -> Option<i64> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let has_unit = chars.iter().any(|&c| cn_unit_value(c).is_some());
    let all_digits = chars.iter().all(|&c| cn_digit_value(c).is_some());

    // Positional mode (no units, all digit chars): 二〇二六 → 2026
    if !has_unit && all_digits {
        let mut n: i64 = 0;
        for c in chars {
            n = n * 10 + cn_digit_value(c).unwrap();
        }
        return Some(n);
    }
    if !has_unit {
        return None;
    }

    // Unit mode: state machine over (digit | small_unit | big_unit).
    let mut total: i64 = 0;
    let mut section: i64 = 0;
    let mut current: i64 = 0;
    for c in chars {
        if let Some(d) = cn_digit_value(c) {
            current = d;
        } else if let Some(u) = cn_unit_value(c) {
            if u >= 10_000 {
                let val = section + current;
                let val = if val == 0 { 1 } else { val };
                total += val * u;
                section = 0;
                current = 0;
            } else {
                let val = if current == 0 { 1 } else { current };
                section += val * u;
                current = 0;
            }
        } else {
            return None;
        }
    }
    Some(total + section + current)
}

fn cn_numeral_token_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[〇零一二三四五六七八九两十百千万亿]+").unwrap())
}

/// Multi-char idioms that look like numerals but are not. Length-2 unit-then-digit
/// or unit-then-unit forms slip past the "≥2 chars" rule, so we exclude them
/// explicitly. Keep this list short — it must only contain truly common cases.
const CN_NUMERAL_IDIOM_SKIP: &[&str] = &["万一", "千万", "万万", "九九"];

/// Convert connected CJK-numeral substrings (length ≥ 2) to Arabic digits.
/// Single-character occurrences are left alone — this naturally skips idioms
/// like 一些 / 二话不说 / 三明治 / 九点 (孤立的数字字)，再用 idiom 白名单兜住
/// "单位+数字"型的"万一/千万"等。
pub fn chinese_numerals_to_arabic(text: &str) -> String {
    cn_numeral_token_re()
        .replace_all(text, |caps: &regex::Captures| {
            let token = &caps[0];
            if token.chars().count() < 2 {
                return token.to_string();
            }
            if CN_NUMERAL_IDIOM_SKIP.iter().any(|w| *w == token) {
                return token.to_string();
            }
            match parse_cn_numeral(token) {
                Some(n) => n.to_string(),
                None => token.to_string(),
            }
        })
        .into_owned()
}

fn cn_version_token_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"[〇零一二三四五六七八九两十百千万亿]+(?:点[〇零一二三四五六七八九两十百千万亿]+){2,}",
        )
        .unwrap()
    })
}

/// Convert CJK version-number patterns like 零点六点零 → 0.6.0.
/// Requires ≥2 "点" separators (≥3 segments) to avoid false positives on
/// 下午两点 / 一点小事 / 三点水 / 版本二点零 etc. where 点 is not a decimal
/// separator or the two-segment form is ambiguous with common expressions.
pub fn chinese_version_numbers_to_arabic(text: &str) -> String {
    cn_version_token_re()
        .replace_all(text, |caps: &regex::Captures| {
            let token = &caps[0];
            let mut parts: Vec<String> = Vec::new();
            for seg in token.split('点') {
                match parse_cn_numeral(seg) {
                    Some(n) => parts.push(n.to_string()),
                    None => return token.to_string(),
                }
            }
            parts.join(".")
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
// process_text — pipeline integration
// ---------------------------------------------------------------------------

/// Apply replacements → CJK numerals → CJK/ASCII spacing → optional capitalize.
/// The numeral conversion and spacing are always-on formatting steps.
pub fn process_text(
    text: &str,
    replacements: &[(String, String)],
    auto_capitalize: bool,
) -> String {
    info!("[text_processor] process_text input={} auto_capitalize={}", mask::mask_text(text), auto_capitalize);
    let after_replacements = apply_replacements(text, replacements);
    let after_letter_merge = merge_uppercase_letter_sequences(&after_replacements);
    let after_version = chinese_version_numbers_to_arabic(&after_letter_merge);
    let after_numerals = chinese_numerals_to_arabic(&after_version);
    let after_spacing = add_cjk_spacing(&after_numerals);
    let result = if auto_capitalize {
        capitalize_sentences(&after_spacing)
    } else {
        after_spacing
    };
    info!("[text_processor] process_text output={}", mask::mask_text(&result));
    result
}
