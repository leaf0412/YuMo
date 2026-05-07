//! 中文数字识别 - 场景模板 + 量词锚点 双层架构
//!
//! 设计文档: _docs/specs/2026-05-07-cn-numerals-redesign-design.md

use regex::Regex;
use std::sync::OnceLock;

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

/// 正则字符类：匹配任意一个中文数字字（与 CN_DIGIT_CHARS 保持同步）。
const CN_DIGIT_CLASS: &str = r"[〇零一二三四五六七八九两十百千万亿]+";

fn is_cn_digit_char(c: char) -> bool {
    CN_DIGIT_CHARS.contains(c)
}

/// 单字量词表。
/// 注意：'两' 不在此列——它是 CN_DIGIT_CHARS 中的数字（值=2），
/// 混入量词表会导致 quantifier_scan 字节追踪下溢 panic。
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
    '米', '斤', '克', '吨', '磅', '升', '度', '伏', '瓦',
    // 次序/名次/容器
    '次', '遍', '趟', '回', '场', '盘', '局',
    '名', '位', '排', '等', '级',
    '杯', '瓶', '罐', '盒', '包', '袋', '箱',
    // 动量词（需配合黑名单过滤伪量词搭配，如"一下"/"一举"）
    '下', '举',
];

/// 伪量词黑名单：(数字字, 量词首字) 二元组命中则跳过
/// 起步只覆盖 "一" + 副词/固定搭配
/// 注意: ('一', '两') 不需要——Task 2 fix 已把 '两' 移出 QUANTIFIERS_SINGLE
const PSEUDO_QUANTIFIER_BLACKLIST: &[(char, char)] = &[
    ('一', '下'),
    ('一', '直'),
    ('一', '定'),
    ('一', '律'),
    ('一', '边'),
    ('一', '概'),
    ('一', '向'),
    ('一', '旦'),
    ('一', '举'),
    ('一', '度'),
];

fn is_blacklisted(span_last: char, q_first: char) -> bool {
    PSEUDO_QUANTIFIER_BLACKLIST
        .iter()
        .any(|&(d, q)| d == span_last && q == q_first)
}

/// 多字量词表（按长度 desc 排序，长匹配优先）。
/// 若未来新增 3 字量词（如"立方米"），需放在 2 字量词之前。
const QUANTIFIERS_MULTI: &[&str] = &[
    // 长度
    "厘米", "毫米", "公里", "千米", "英里", "英尺", "英寸",
    // 重量
    "公斤", "千克", "毫克",
    // 体积
    "毫升", "加仑",
    // 物理
    "瓦特", "赫兹",
    // 时间
    "小时", "分钟", "钟头", "星期", "季度",
];

fn is_single_quantifier(c: char) -> bool {
    QUANTIFIERS_SINGLE.contains(&c)
}

/// 在 chars[i..] 起点匹配最长量词，返回量词字符长度（None 表示未命中）。
/// 多字量词优先（QUANTIFIERS_MULTI 按长度 desc），命中后直接返回；
/// 无多字命中再检查单字。
fn match_quantifier_at(chars: &[char], i: usize) -> Option<usize> {
    for q in QUANTIFIERS_MULTI {
        let q_chars: Vec<char> = q.chars().collect();
        let q_len = q_chars.len();
        if i + q_len <= chars.len() && chars[i..i + q_len] == q_chars[..] {
            return Some(q_len);
        }
    }
    if is_single_quantifier(chars[i]) {
        return Some(1);
    }
    None
}

/// 版本号正则：匹配 ≥3 段中文数字以 '点' 分隔的整体（如"零点六点零"）。
/// 必须先于小数模板应用，否则前两段会被小数模板偷吃。
fn cn_version_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(r"{0}(?:点{0}){{2,}}", CN_DIGIT_CLASS)).unwrap()
    })
}

/// 小数正则：匹配 X点Y，左右各至少一个中文数字字。
fn cn_decimal_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(r"({0})点({0})", CN_DIGIT_CLASS)).unwrap()
    })
}

/// 版本号模板：零点六点零 → 0.6.0，一点二点三点四 → 1.2.3.4。
/// 任一段解析失败则保留原文（不兜底）。
fn apply_version_template(text: &str) -> String {
    cn_version_re()
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

/// 小数模板：二点五 → 2.5，十二点三四 → 12.34。
/// 约束：左+右合计 ≥2 个中文数字字，避免单字-点-单字歧义被误转。
/// 实际上 CN_DIGIT_CLASS 已要求每侧 ≥1 字，"单字+单字"=2 ≥2 仍可命中——
/// 此约束保留以应对未来 regex 可能放宽为 `*` 的情况，并作文档说明。
/// 解析失败保留原文（不兜底）。
fn apply_decimal_template(text: &str) -> String {
    cn_decimal_re()
        .replace_all(text, |caps: &regex::Captures| {
            let left = &caps[1];
            let right = &caps[2];
            let total_chars = left.chars().count() + right.chars().count();
            if total_chars < 2 {
                return caps[0].to_string();
            }
            match (parse_cn_numeral(left), parse_cn_numeral(right)) {
                (Some(l), Some(r)) => format!("{}.{}", l, r),
                _ => caps[0].to_string(),
            }
        })
        .into_owned()
}

fn cn_ordinal_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!(r"第({})", CN_DIGIT_CLASS)).unwrap())
}

fn cn_negative_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!(r"负({})", CN_DIGIT_CLASS)).unwrap())
}

/// 序数模板：第三 → 第3，第二十五 → 第25。
/// parse_cn_numeral 返回 None 时保留原文（不兜底）。
fn apply_ordinal_template(text: &str) -> String {
    cn_ordinal_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_cn_numeral(&caps[1]) {
                Some(n) => format!("第{}", n),
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 负数模板：负三百 → -300，负二十 → -20。
/// parse_cn_numeral 返回 None 时保留原文（不兜底）。
fn apply_negative_template(text: &str) -> String {
    cn_negative_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_cn_numeral(&caps[1]) {
                Some(n) => format!("-{}", n),
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 扫描文本，遇到量词锚点则往左回溯中文数字字，调用 parse_cn_numeral 转换。
/// 支持单字量词（个/克/年 等）与多字量词（公里/小时/厘米 等），多字优先长匹配。
fn quantifier_scan(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if let Some(q_len) = match_quantifier_at(&chars, i) {
            // 从量词起点 i 往左回溯连续中文数字字
            let mut j = i;
            while j > 0 && is_cn_digit_char(chars[j - 1]) {
                j -= 1;
            }
            if j < i {
                let span: String = chars[j..i].iter().collect();
                let span_last = chars[i - 1];
                let q_first = chars[i];
                if !is_blacklisted(span_last, q_first) {
                    if let Some(num) = parse_cn_numeral(&span) {
                        // 撤销 out 中已写入的 span（中文字符 UTF-8 是 3 字节，不能按 char 数 truncate）
                        let span_bytes: usize = chars[j..i].iter().map(|ch| ch.len_utf8()).sum();
                        out.truncate(out.len() - span_bytes);
                        out.push_str(&num.to_string());
                        for k in 0..q_len {
                            out.push(chars[i + k]);
                        }
                        i += q_len;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn convert_cn_numerals(text: &str) -> String {
    // Pipeline 顺序遵循优先级：
    // 版本号 (≥3段) > 小数 > 负数 > 序数 > 量词扫描。
    // Task 7 加百分/千分/分数后将插入到版本号后、小数前。
    let s = apply_version_template(text);
    let s = apply_decimal_template(&s);
    let s = apply_negative_template(&s);
    let s = apply_ordinal_template(&s);
    quantifier_scan(&s)
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

    /// 不变量：CN_DIGIT_CLASS 必须能匹配 CN_DIGIT_CHARS 中的每个字符。
    /// 两者独立维护，加新字符时容易漏改一边——此测试守住它们的字符集相等。
    #[test]
    fn invariant_digit_class_matches_digit_chars() {
        let re = Regex::new(&format!("^{}$", CN_DIGIT_CLASS)).unwrap();
        for c in CN_DIGIT_CHARS.chars() {
            assert!(
                re.is_match(&c.to_string()),
                "CN_DIGIT_CLASS 缺少字符: {:?}",
                c
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

    #[test]
    fn quantifier_scan_multi_char_quantifier() {
        assert_eq!(convert_cn_numerals("跑了三公里"), "跑了3公里");
        assert_eq!(convert_cn_numerals("等了两小时"), "等了2小时");
        assert_eq!(convert_cn_numerals("二十厘米"), "20厘米");
    }

    #[test]
    fn quantifier_scan_long_match_priority() {
        // "公里" 优先于 "里"（如 "里" 进单字量词）；"小时" 优先于 "时"
        assert_eq!(convert_cn_numerals("十公里"), "10公里");
    }

    /// 回归：位值记法年份（二〇二六年）→ 2026年。
    #[test]
    fn quantifier_scan_positional_year() {
        assert_eq!(convert_cn_numerals("二〇二六年"), "2026年");
    }

    #[test]
    fn pseudo_quantifier_blacklist() {
        // "一" + 后随字 是副词 / 固定搭配，不转
        assert_eq!(convert_cn_numerals("一下子"), "一下子");
        assert_eq!(convert_cn_numerals("一直走"), "一直走");
        assert_eq!(convert_cn_numerals("一定可以"), "一定可以");
        assert_eq!(convert_cn_numerals("一概而论"), "一概而论");
        assert_eq!(convert_cn_numerals("一向如此"), "一向如此");
        assert_eq!(convert_cn_numerals("一旦发生"), "一旦发生");
        assert_eq!(convert_cn_numerals("一举两得"), "一举两得");
        assert_eq!(convert_cn_numerals("一度过严寒"), "一度过严寒");
        // 防御性: '两' 当前不在 QUANTIFIERS_SINGLE，量词扫描不命中 — trivially pass
        // 若将来 '两' 加回量词表，需同步加 ('一','两') 到 PSEUDO_QUANTIFIER_BLACKLIST
        assert_eq!(convert_cn_numerals("一两重"), "一两重");
    }

    #[test]
    fn pseudo_quantifier_does_not_block_multi_digit() {
        // 多字数字 + 同样的伪量词字仍然转
        assert_eq!(convert_cn_numerals("二十下"), "20下");
        assert_eq!(convert_cn_numerals("十度"), "10度");
    }

    #[test]
    fn template_ordinal() {
        assert_eq!(convert_cn_numerals("第三"), "第3");
        assert_eq!(convert_cn_numerals("第二十五"), "第25");
        assert_eq!(convert_cn_numerals("第一名"), "第1名"); // 与量词扫描不冲突
    }

    #[test]
    fn template_negative() {
        assert_eq!(convert_cn_numerals("负三百"), "-300");
        assert_eq!(convert_cn_numerals("温度负二十度"), "温度-20度");
    }

    #[test]
    fn template_decimal() {
        assert_eq!(convert_cn_numerals("二点五"), "2.5");
        assert_eq!(convert_cn_numerals("零点六"), "0.6");
        assert_eq!(convert_cn_numerals("十二点三四"), "12.34");
    }

    #[test]
    fn template_decimal_skips_ambiguous_two_chars() {
        // 小数模板不命中 "两点钟"（'钟' 不是 CN_DIGIT_CLASS 字符）
        // 但量词扫描会把 "两点" 转成 "2点"（与 design 4.2 对齐：九点 → 9点）
        assert_eq!(convert_cn_numerals("下午两点钟"), "下午2点钟");
        // "两点九" 合计 2 字，小数模板命中
        assert_eq!(convert_cn_numerals("两点九"), "2.9");
    }

    #[test]
    fn template_version() {
        assert_eq!(convert_cn_numerals("零点六点零"), "0.6.0");
        assert_eq!(convert_cn_numerals("一点二点三点四"), "1.2.3.4");
        assert_eq!(convert_cn_numerals("二十点一点零"), "20.1.0");
    }

    #[test]
    fn template_version_priority_over_decimal() {
        // 版本号 ≥3 段必须先匹配，整体转成 X.Y.Z 而不是被小数模板拆成 "X.Y点Z"
        assert_eq!(convert_cn_numerals("发布零点六点零的版本"), "发布0.6.0的版本");
    }
}
