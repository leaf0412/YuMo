//! cn_numerals 单元测试。
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

#[test]
fn template_decimal_preserves_leading_zeros_in_fraction() {
    // 中文小数位按位读：零点零五 = 0.05，不是 0.5
    assert_eq!(convert_cn_numerals("零点零五"), "0.05");
    assert_eq!(convert_cn_numerals("一点零六"), "1.06");
    assert_eq!(convert_cn_numerals("零点零零五"), "0.005");
    // 多位数字串保持
    assert_eq!(convert_cn_numerals("二点三零四"), "2.304");
}

#[test]
fn template_decimal_right_with_unit_is_invalid() {
    // 右侧含单位字（十）不是合法中文小数：小数模板跳过，保留 "二点十"。
    // 但 pipeline 后续的 quantifier_scan 会识别 '点' 为量词锚点，
    // 将 "二" 转成 "2"（与 "下午两点钟" → "下午2点钟" 行为一致）。
    // 因此全 pipeline 输出为 "2点十"，而非"二点十"。
    assert_eq!(convert_cn_numerals("二点十"), "2点十");
}

#[test]
fn template_percent() {
    assert_eq!(convert_cn_numerals("百分之三十"), "30%");
    assert_eq!(convert_cn_numerals("百分之一点五"), "1.5%");
    assert_eq!(convert_cn_numerals("增长百分之二十"), "增长20%");
}

#[test]
fn template_percent_with_leading_zero_decimal() {
    // 小数右侧按位（继承 Task 6 fix）
    assert_eq!(convert_cn_numerals("百分之零点零五"), "0.05%");
}

#[test]
fn template_permille() {
    assert_eq!(convert_cn_numerals("千分之五"), "5‰");
    assert_eq!(convert_cn_numerals("千分之零点八"), "0.8‰");
}

#[test]
fn template_fraction() {
    assert_eq!(convert_cn_numerals("三分之一"), "1/3");
    assert_eq!(convert_cn_numerals("五分之二"), "2/5");
    assert_eq!(convert_cn_numerals("吃了三分之一"), "吃了1/3");
}

#[test]
fn template_percent_takes_priority_over_fraction() {
    // "百分之X" 必须先被 percent 吃掉，否则 fraction 会把它误解为 "百/分之/X" → "X/100" 格式
    // 验证输出格式（带 % 而非 / 表达），即 percent 优先级更高
    let result = convert_cn_numerals("百分之三十");
    assert_eq!(result, "30%");
    assert!(!result.contains("/100"), "fraction 模板抢吃了百分比模板");

    // 同样：百分之一点五（含小数）也走 percent 而非 fraction
    let result = convert_cn_numerals("百分之一点五");
    assert_eq!(result, "1.5%");
    assert!(!result.contains("/"), "fraction 模板抢吃了百分比+小数");
}

#[test]
fn template_negative_with_decimal() {
    // 负数支持小数（design §4.1 #5: 负二点五 → -2.5）
    assert_eq!(convert_cn_numerals("负二点五"), "-2.5");
    // 负数 + 含前导零小数
    assert_eq!(convert_cn_numerals("负零点零五"), "-0.05");
}
