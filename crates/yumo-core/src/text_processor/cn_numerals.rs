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

const CN_DIGIT_CHARS: &str = "〇零一二三四五六七八九两十百千万亿";

fn is_cn_digit_char(c: char) -> bool {
    CN_DIGIT_CHARS.contains(c)
}

/// 起步量词表（单字）。多字量词在 Task 3 加。
const QUANTIFIERS_SINGLE: &[char] = &[
    // 时间
    '年', '月', '日', '号', '点', '分', '秒', '天', '周',
    // 货币
    '块', '元', '角', '毛',
    // 通用计量
    '个', '只', '条', '根', '颗', '粒', '张', '把', '支',
    '双', '对', '副', '件', '部', '辆', '座', '层', '楼',
    '页', '段', '篇',
    // 长度/重量/体积/物理（单字部分）
    // 注意：'两' 不在此列，它同时是 CN_DIGIT_CHARS 中的数字（值=2），
    // 混入量词表会导致 quantifier_scan 字节追踪下溢 panic。
    '米', '斤', '克', '吨', '磅', '升', '度', '伏', '瓦',
    // 次序/名次/容器
    '次', '遍', '趟', '回', '场', '盘', '局',
    '名', '位', '排', '等', '级',
    '杯', '瓶', '罐', '盒', '包', '袋', '箱',
];

fn is_single_quantifier(c: char) -> bool {
    QUANTIFIERS_SINGLE.contains(&c)
}

/// 扫描文本，遇到量词锚点则往左回溯中文数字字，调用 parse_cn_numeral 转换。
fn quantifier_scan(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if is_single_quantifier(c) {
            // 从 i 往左回溯连续中文数字字
            let mut j = i;
            while j > 0 && is_cn_digit_char(chars[j - 1]) {
                j -= 1;
            }
            if j < i {
                let span: String = chars[j..i].iter().collect();
                if let Some(num) = parse_cn_numeral(&span) {
                    // 撤销 out 中已写入的 span（中文字符 UTF-8 是 3 字节，不能按 char 数 truncate）
                    let span_bytes: usize = chars[j..i].iter().map(|ch| ch.len_utf8()).sum();
                    out.truncate(out.len() - span_bytes);
                    out.push_str(&num.to_string());
                    out.push(c);
                    i += 1;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

pub fn convert_cn_numerals(text: &str) -> String {
    quantifier_scan(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_passthrough_empty() {
        assert_eq!(convert_cn_numerals(""), "");
    }

    #[test]
    fn convert_passthrough_no_cn() {
        assert_eq!(convert_cn_numerals("hello world"), "hello world");
    }

    #[test]
    fn quantifier_scan_single_digit() {
        assert_eq!(convert_cn_numerals("三个"), "3个");
        assert_eq!(convert_cn_numerals("我有五块"), "我有5块");
    }

    #[test]
    fn quantifier_scan_no_anchor_no_change() {
        // 没量词锚点 → 不转
        assert_eq!(convert_cn_numerals("三明治"), "三明治");
        assert_eq!(convert_cn_numerals("一些事"), "一些事");
    }

    /// 不变量：QUANTIFIERS_SINGLE 与 CN_DIGIT_CHARS 必须不相交。
    /// 若有交集，quantifier_scan 的字节追踪会因已替换字符长度变化而下溢 panic。
    #[test]
    fn invariant_quantifiers_disjoint_from_digits() {
        for q in QUANTIFIERS_SINGLE {
            assert!(
                !CN_DIGIT_CHARS.contains(*q),
                "QUANTIFIERS_SINGLE 含中文数字字 {:?}, 会导致 quantifier_scan 字节追踪错乱",
                q
            );
        }
    }

    /// 回归：'两' 是 cn_digit 但不是 quantifier，扫到下个量词时应正确回溯+解析，不 panic。
    #[test]
    fn quantifier_scan_does_not_panic_on_两_in_span() {
        // "一两" → parse_cn_numeral 走 positional → 12，'克' 是量词
        assert_eq!(convert_cn_numerals("一两克"), "12克");
        // 单独 '两' 后跟量词
        assert_eq!(convert_cn_numerals("两个"), "2个");
        // "三两条"：span="三两"，positional → 32，'条' 是量词
        assert_eq!(convert_cn_numerals("三两条"), "32条");
    }

    /// 回归：多字 span 的 byte truncate 路径（每个中文字 3 bytes）。
    #[test]
    fn quantifier_scan_multi_digit_truncate_path() {
        assert_eq!(convert_cn_numerals("二十个"), "20个");
        assert_eq!(convert_cn_numerals("一百名"), "100名");
        assert_eq!(convert_cn_numerals("一千零五次"), "1005次");
    }

    /// 回归：同一字符串中多个量词锚点各自正确截断+转换。
    #[test]
    fn quantifier_scan_multiple_in_one_string() {
        assert_eq!(convert_cn_numerals("三个五块"), "3个5块");
        assert_eq!(convert_cn_numerals("我有三块他有五个"), "我有3块他有5个");
    }
}
