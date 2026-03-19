use crate::mask;
use log::info;
use regex::RegexBuilder;

/// Apply word-boundary-aware, case-insensitive replacements.
pub fn apply_replacements(text: &str, replacements: &[(String, String)]) -> String {
    info!("[text_processor] apply_replacements text_length={} replacements_count={}", text.len(), replacements.len());
    let mut result = text.to_string();
    for (original, replacement) in replacements {
        let pattern = format!(r"\b{}\b", regex::escape(original));
        if let Ok(re) = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
        {
            result = re.replace_all(&result, replacement.as_str()).to_string();
        }
    }
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

    result
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
