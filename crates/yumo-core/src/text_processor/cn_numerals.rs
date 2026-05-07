//! 中文数字识别 - 场景模板 + 量词锚点 双层架构
//!
//! 设计文档: _docs/specs/2026-05-07-cn-numerals-redesign-design.md

use log::info;
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
    '年', '月', '日', '号', '点', '分', '秒', '天', '周', '岁',
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
            let result = parts.join(".");
            info!(
                "[text_processor::cn_num] template_match scene=version span={:?} -> {:?}",
                token, result
            );
            result
        })
        .into_owned()
}

/// 把"X点Y"中的 Y 部分按位转为阿拉伯数字串。
/// 中文小数位是按位读出的（"零五" = "05"，不是整数 5），因此不能用
/// parse_cn_numeral——那会把 "零五" 解析为整数 5，丢失前导零。
/// 若 Y 中含单位字（十/百/千/万/亿），则不是合法小数位，返回 None。
fn cn_positional_digits_to_str(s: &str) -> Option<String> {
    s.chars()
        .map(|c| cn_digit_value(c).map(|v| v.to_string()))
        .collect::<Option<Vec<_>>>()
        .map(|v| v.join(""))
}

/// 小数模板：二点五 → 2.5，十二点三四 → 12.34，零点零五 → 0.05。
/// 左侧用 parse_cn_numeral（支持 "十二" 等位值写法）；
/// 右侧用 cn_positional_digits_to_str（按位拼接，保前导零）。
/// 右侧含单位字（"二点十"）→ None，保留原文（不兜底）。
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
            match (parse_cn_numeral(left), cn_positional_digits_to_str(right)) {
                (Some(l), Some(r)) => {
                    let result = format!("{}.{}", l, r);
                    info!(
                        "[text_processor::cn_num] template_match scene=decimal span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
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
    RE.get_or_init(|| {
        Regex::new(&format!(r"负({0}(?:点{0})?)", CN_DIGIT_CLASS)).unwrap()
    })
}

fn cn_percent_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(r"百分之({0}(?:点{0})?)", CN_DIGIT_CLASS)).unwrap()
    })
}

fn cn_permille_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(r"千分之({0}(?:点{0})?)", CN_DIGIT_CLASS)).unwrap()
    })
}

fn cn_fraction_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(r"({0})分之({0})", CN_DIGIT_CLASS)).unwrap()
    })
}

/// 把"整数"或"整数点整数"形式的中文数字串解析为字符串形式。
/// 整数走 parse_cn_numeral；小数右侧按位（继承 Task 6 fix）。
/// 任一段失败返回 None。
fn parse_int_or_decimal(s: &str) -> Option<String> {
    if let Some((left, right)) = s.split_once('点') {
        let l = parse_cn_numeral(left)?;
        let r = cn_positional_digits_to_str(right)?;
        Some(format!("{}.{}", l, r))
    } else {
        parse_cn_numeral(s).map(|n| n.to_string())
    }
}

/// 百分比模板：百分之三十 → 30%，百分之一点五 → 1.5%。
/// 数字部分支持整数或小数（递归走 parse_int_or_decimal）。
fn apply_percent_template(text: &str) -> String {
    cn_percent_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_int_or_decimal(&caps[1]) {
                Some(n) => {
                    let result = format!("{}%", n);
                    info!(
                        "[text_processor::cn_num] template_match scene=percent span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 千分比模板：千分之五 → 5‰，千分之零点八 → 0.8‰。
fn apply_permille_template(text: &str) -> String {
    cn_permille_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_int_or_decimal(&caps[1]) {
                Some(n) => {
                    let result = format!("{}‰", n);
                    info!(
                        "[text_processor::cn_num] template_match scene=permille span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 分数模板：三分之一 → 1/3。
/// 注：分母在前 (caps[1])、分子在后 (caps[2])，按中文表达「X分之Y」=Y/X。
fn apply_fraction_template(text: &str) -> String {
    cn_fraction_re()
        .replace_all(text, |caps: &regex::Captures| {
            match (parse_cn_numeral(&caps[1]), parse_cn_numeral(&caps[2])) {
                (Some(denom), Some(numer)) => {
                    let result = format!("{}/{}", numer, denom);
                    info!(
                        "[text_processor::cn_num] template_match scene=fraction span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
                _ => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 序数模板：第三 → 第3，第二十五 → 第25。
/// parse_cn_numeral 返回 None 时保留原文（不兜底）。
fn apply_ordinal_template(text: &str) -> String {
    cn_ordinal_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_cn_numeral(&caps[1]) {
                Some(n) => {
                    let result = format!("第{}", n);
                    info!(
                        "[text_processor::cn_num] template_match scene=ordinal span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// 负数模板：负三百 → -300, 负二点五 → -2.5, 负零点零五 → -0.05。
/// 数字部分支持整数或小数（共用 parse_int_or_decimal helper）。
/// parse 失败保留原文（不兜底）。
fn apply_negative_template(text: &str) -> String {
    cn_negative_re()
        .replace_all(text, |caps: &regex::Captures| {
            match parse_int_or_decimal(&caps[1]) {
                Some(n) => {
                    let result = format!("-{}", n);
                    info!(
                        "[text_processor::cn_num] template_match scene=negative span={:?} -> {:?}",
                        &caps[0], result
                    );
                    result
                }
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
                if is_blacklisted(span_last, q_first) {
                    info!(
                        "[text_processor::cn_num] skip span={:?} anchor={:?} reason=pseudo_quantifier_blacklist",
                        span_last, q_first
                    );
                } else if let Some(num) = parse_cn_numeral(&span) {
                    // 撤销 out 中已写入的 span（中文字符 UTF-8 是 3 字节，不能按 char 数 truncate）
                    let span_bytes: usize = chars[j..i].iter().map(|ch| ch.len_utf8()).sum();
                    out.truncate(out.len() - span_bytes);
                    let q_str: String = chars[i..i + q_len].iter().collect();
                    info!(
                        "[text_processor::cn_num] quantifier_match anchor={:?} span={:?} -> {}",
                        q_str, span, num
                    );
                    out.push_str(&num.to_string());
                    out.push_str(&q_str);
                    i += q_len;
                    continue;
                } else {
                    info!(
                        "[text_processor::cn_num] parse_failed span={:?} anchor={:?}",
                        span, q_first
                    );
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn convert_cn_numerals(text: &str) -> String {
    // Pipeline 顺序遵循 design §4.1 优先级（外层壳先于内层数字）：
    // 版本号 (≥3段) > 百分比 > 千分比 > 分数 > 负数 > 小数 > 序数 > 量词扫描。
    // 关键约束:
    // - 百分/千分/分数 必须先于小数（外层"百分之X"消费后避免内部"X点Y"被偷吃）
    // - 负数模板自身吸收小数尾巴（递归走 parse_int_or_decimal），不再依赖 decimal 前置
    let s = apply_version_template(text);
    let s = apply_percent_template(&s);
    let s = apply_permille_template(&s);
    let s = apply_fraction_template(&s);
    let s = apply_negative_template(&s);
    let s = apply_decimal_template(&s);
    let s = apply_ordinal_template(&s);
    quantifier_scan(&s)
}

#[cfg(test)]
mod tests;
