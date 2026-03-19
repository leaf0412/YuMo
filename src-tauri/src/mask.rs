/// Mask sensitive strings for logging: show first 4 and last 4 characters.
/// Short strings (≤ 10 chars) show only first 2 + last 2.
/// Empty/very short strings show "***".
pub fn mask(s: &str) -> String {
    let len = s.len();
    if len <= 4 {
        return "***".to_string();
    }
    if len <= 10 {
        format!("{}...{}", &s[..2], &s[len - 2..])
    } else {
        format!("{}...{}", &s[..4], &s[len - 4..])
    }
}

/// Mask text content for logging: show first 20 chars + length.
pub fn mask_text(s: &str) -> String {
    let len = s.chars().count();
    if len <= 20 {
        format!("\"{}\" ({}chars)", s, len)
    } else {
        let preview: String = s.chars().take(20).collect();
        format!("\"{}...\" ({}chars)", preview, len)
    }
}
