use crate::mask;
use log::{info, warn};
use regex::{Regex, RegexBuilder};

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

/// Apply replacements first, then optionally capitalize sentences.
pub fn process_text(
    text: &str,
    replacements: &[(String, String)],
    auto_capitalize: bool,
) -> String {
    info!("[text_processor] process_text input={} auto_capitalize={}", mask::mask_text(text), auto_capitalize);
    let after_replacements = apply_replacements(text, replacements);
    let result = if auto_capitalize {
        capitalize_sentences(&after_replacements)
    } else {
        after_replacements
    };
    info!("[text_processor] process_text output={}", mask::mask_text(&result));
    result
}
