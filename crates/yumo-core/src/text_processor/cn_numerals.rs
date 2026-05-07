//! 中文数字识别 - 场景模板 + 量词锚点 双层架构
//!
//! 设计文档: _docs/specs/2026-05-07-cn-numerals-redesign-design.md

pub(super) fn cn_digit_value(c: char) -> Option<i64> {
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

pub(super) fn cn_unit_value(c: char) -> Option<i64> {
    match c {
        '十' => Some(10),
        '百' => Some(100),
        '千' => Some(1000),
        '万' => Some(10_000),
        '亿' => Some(100_000_000),
        _ => None,
    }
}

pub(super) fn parse_cn_numeral(s: &str) -> Option<i64> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let has_unit = chars.iter().any(|&c| cn_unit_value(c).is_some());
    let all_digits = chars.iter().all(|&c| cn_digit_value(c).is_some());

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
