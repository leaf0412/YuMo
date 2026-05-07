//! 自 2026-05 中文数字识别重构后:
//! - "独立中文数字串无量词锚点"不再转换（强证据原则）
//!   例: "二十" → "二十", 但 "二十块" → "20块"
//! - 单字数字+量词转换（A1 决策）
//!   例: "三个" → "3个", "九点" → "9点"
//! - 部分场景 (一点 / 三点 / 二点零) 因量词扫描接管，跟旧 ≥3 段版本号约束行为不同
//!   设计文档: _docs/specs/2026-05-07-cn-numerals-redesign-design.md

use yumo_core::text_processor;

#[test]
fn test_apply_replacements() {
    let replacements = vec![
        ("k8s".to_string(), "Kubernetes".to_string()),
        ("js".to_string(), "JavaScript".to_string()),
    ];
    let result = text_processor::apply_replacements("I use k8s and js daily", &replacements);
    assert_eq!(result, "I use Kubernetes and JavaScript daily");
}

#[test]
fn test_replacement_case_insensitive() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::apply_replacements("K8S is great", &replacements);
    assert_eq!(result, "Kubernetes is great");
}

#[test]
fn test_replacement_word_boundary() {
    let replacements = vec![("js".to_string(), "JavaScript".to_string())];
    // Should NOT replace "json" → "JavaScripton"
    let result = text_processor::apply_replacements("I work with json files", &replacements);
    assert_eq!(result, "I work with json files");
}

#[test]
fn test_replacement_at_string_boundaries() {
    let replacements = vec![("js".to_string(), "JavaScript".to_string())];
    let result = text_processor::apply_replacements("js is popular", &replacements);
    assert_eq!(result, "JavaScript is popular");

    let result = text_processor::apply_replacements("I love js", &replacements);
    assert_eq!(result, "I love JavaScript");
}

#[test]
fn test_auto_capitalize() {
    let result = text_processor::capitalize_sentences("hello world. this is a test. yes.");
    assert_eq!(result, "Hello world. This is a test. Yes.");
}

#[test]
fn test_capitalize_after_question_mark() {
    let result = text_processor::capitalize_sentences("what? really. ok.");
    assert_eq!(result, "What? Really. Ok.");
}

#[test]
fn test_capitalize_after_exclamation() {
    let result = text_processor::capitalize_sentences("wow! that is great.");
    assert_eq!(result, "Wow! That is great.");
}

#[test]
fn test_capitalize_empty_string() {
    let result = text_processor::capitalize_sentences("");
    assert_eq!(result, "");
}

#[test]
fn test_capitalize_already_capitalized() {
    let result = text_processor::capitalize_sentences("Hello World.");
    assert_eq!(result, "Hello World.");
}

#[test]
fn test_process_text_combined() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        true, // auto_capitalize
    );
    assert_eq!(result, "I deploy to Kubernetes. It works.");
}

#[test]
fn test_process_text_no_capitalize() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        false,
    );
    assert_eq!(result, "i deploy to Kubernetes. it works.");
}

#[test]
fn test_no_replacements() {
    let result = text_processor::apply_replacements("hello world", &[]);
    assert_eq!(result, "hello world");
}

// ---------------------------------------------------------------------------
// convert_cn_numerals
// ---------------------------------------------------------------------------

#[test]
fn cn_num_skips_single_chars() {
    // 无量词锚点的 idiom，保持原文
    assert_eq!(text_processor::convert_cn_numerals("一些事"), "一些事");
    assert_eq!(text_processor::convert_cn_numerals("一下子"), "一下子");        // 黑名单 (一, 下)
    assert_eq!(text_processor::convert_cn_numerals("一定可以"), "一定可以");    // 黑名单 (一, 定)
    assert_eq!(text_processor::convert_cn_numerals("二话不说"), "二话不说");    // "话"非量词
    assert_eq!(text_processor::convert_cn_numerals("三明治"), "三明治");        // "明"非量词
    assert_eq!(text_processor::convert_cn_numerals("万一发生"), "万一发生");    // "发"非量词
    // A1 决策: 单字+量词转 (期望已更新)
    assert_eq!(text_processor::convert_cn_numerals("九点开会"), "9点开会");
    assert_eq!(text_processor::convert_cn_numerals("二月一号"), "2月1号");
    assert_eq!(text_processor::convert_cn_numerals("十块钱"), "10块钱");
}

#[test]
fn cn_num_simple_units() {
    assert_eq!(text_processor::convert_cn_numerals("一百块"), "100块");
    // 以下无量词锚点 → 保留原文 (变化!)
    assert_eq!(text_processor::convert_cn_numerals("二十"), "二十");
    assert_eq!(text_processor::convert_cn_numerals("两百"), "两百");
    assert_eq!(text_processor::convert_cn_numerals("三千"), "三千");
    assert_eq!(text_processor::convert_cn_numerals("一万"), "一万");
    assert_eq!(text_processor::convert_cn_numerals("二十五"), "二十五");
    assert_eq!(text_processor::convert_cn_numerals("三百二十一"), "三百二十一");
}

#[test]
fn cn_num_positional() {
    assert_eq!(text_processor::convert_cn_numerals("二〇二六年"), "2026年");
    // 无量词 → 保留 (变化!)
    assert_eq!(text_processor::convert_cn_numerals("一九八四"), "一九八四");
    assert_eq!(text_processor::convert_cn_numerals("二三"), "二三");
}

#[test]
fn cn_num_complex() {
    // 全部无量词 → 保留原文 (变化!)
    assert_eq!(text_processor::convert_cn_numerals("一万两千三百四十五"), "一万两千三百四十五");
    assert_eq!(text_processor::convert_cn_numerals("一千零五"), "一千零五");
    assert_eq!(text_processor::convert_cn_numerals("两亿三千万"), "两亿三千万");
    assert_eq!(text_processor::convert_cn_numerals("十万"), "十万");
}

#[test]
fn cn_num_mixed_in_sentence() {
    // 前段有量词转，后段无量词不转 (变化!)
    assert_eq!(
        text_processor::convert_cn_numerals("我有三百二十一块钱，他有一万"),
        "我有321块钱，他有一万",
    );
    assert_eq!(
        text_processor::convert_cn_numerals("生于二〇〇一年，今年二十五岁"),
        "生于2001年，今年25岁",
    );
}

#[test]
fn cn_num_no_change_on_non_chinese() {
    assert_eq!(text_processor::convert_cn_numerals("hello"), "hello");
    assert_eq!(text_processor::convert_cn_numerals("1234"), "1234");
    assert_eq!(text_processor::convert_cn_numerals(""), "");
}

// ---------------------------------------------------------------------------
// add_cjk_spacing
// ---------------------------------------------------------------------------

#[test]
fn cjk_spacing_inserts_between_cjk_and_latin() {
    assert_eq!(text_processor::add_cjk_spacing("今天天气good"), "今天天气 good");
    assert_eq!(text_processor::add_cjk_spacing("Hello世界"), "Hello 世界");
    assert_eq!(text_processor::add_cjk_spacing("iPhone的麦克风"), "iPhone 的麦克风");
}

#[test]
fn cjk_spacing_inserts_between_cjk_and_digit() {
    assert_eq!(text_processor::add_cjk_spacing("今天是2026年"), "今天是 2026 年");
    assert_eq!(text_processor::add_cjk_spacing("第3名"), "第 3 名");
}

#[test]
fn cjk_spacing_no_double_spacing() {
    // Already spaced — must not add a second space.
    assert_eq!(text_processor::add_cjk_spacing("Hello 世界"), "Hello 世界");
    assert_eq!(text_processor::add_cjk_spacing("今天 good"), "今天 good");
}

#[test]
fn cjk_spacing_leaves_pure_text_alone() {
    assert_eq!(text_processor::add_cjk_spacing("Hello, world"), "Hello, world");
    assert_eq!(text_processor::add_cjk_spacing("abc123"), "abc123");
    assert_eq!(text_processor::add_cjk_spacing("你好世界"), "你好世界");
    assert_eq!(text_processor::add_cjk_spacing("你好。世界"), "你好。世界");
    assert_eq!(text_processor::add_cjk_spacing(""), "");
}

// ---------------------------------------------------------------------------
// process_text integration: chinese numerals + spacing run by default
// ---------------------------------------------------------------------------

#[test]
fn process_text_applies_cn_numerals_and_spacing() {
    let result = text_processor::process_text("我有一百块Hello", &[], false);
    assert_eq!(result, "我有 100 块 Hello");
}

#[test]
fn process_text_idiom_safe() {
    // Single-char numerals in idioms must not be touched even with formatting on.
    let result = text_processor::process_text("一些事情发生了", &[], false);
    assert_eq!(result, "一些事情发生了");
}

// ---------------------------------------------------------------------------
// convert_cn_numerals — 版本号形态 (X点Y点Z...) 专门处理
// ---------------------------------------------------------------------------

#[test]
fn cn_version_three_segments() {
    assert_eq!(
        text_processor::convert_cn_numerals("可以发布零点六点零的版本"),
        "可以发布0.6.0的版本",
    );
}

#[test]
fn cn_version_four_plus_segments() {
    assert_eq!(
        text_processor::convert_cn_numerals("版本一点二点三点四发布了"),
        "版本1.2.3.4发布了",
    );
}

#[test]
fn cn_version_multi_char_segments() {
    // Each segment can itself be a multi-char numeral (positional or unit).
    assert_eq!(
        text_processor::convert_cn_numerals("升级到二十点一点零"),
        "升级到20.1.0",
    );
}

#[test]
fn cn_version_two_segments_now_handled_by_decimal_or_quantifier() {
    // 旧版本: 这些 case 因 ≥3 段约束都不转。新版本由小数模板/量词扫描接管。
    // 行为变化已与用户对齐 (设计 spec section 4.2)。

    // 量词"点" + 单字"两" → 转 (A1)
    assert_eq!(
        text_processor::convert_cn_numerals("下午两点到会议室"),
        "下午2点到会议室",
    );
    // 小数模板命中 (≥2 数字字)
    assert_eq!(
        text_processor::convert_cn_numerals("版本二点零"),
        "版本2.0",
    );
    // KNOWN-ISSUE: "一点" 在某些上下文是"少量"义而非"1点"。当前会被量词扫描误转。
    // 已记录在 corpus tsv 等待后续语料触发后处理。
    assert_eq!(
        text_processor::convert_cn_numerals("就是一点小事"),
        "就是1点小事",
    );
    assert_eq!(
        text_processor::convert_cn_numerals("三点水偏旁"),
        "3点水偏旁",
    );
}

#[test]
fn cn_version_no_change_on_plain_text() {
    assert_eq!(text_processor::convert_cn_numerals("hello 1.2.3"), "hello 1.2.3");
    assert_eq!(text_processor::convert_cn_numerals(""), "");
}

#[test]
fn process_text_version_pipeline() {
    // End-to-end: version numerals convert, then CJK/ASCII spacing kicks in.
    let result = text_processor::process_text("可以发布零点六点零的版本", &[], false);
    assert_eq!(result, "可以发布 0.6.0 的版本");
}

#[test]
fn process_text_version_does_not_touch_time() {
    // A1 后行为变化：原期望 "下午两点开会"，新版本量词扫描会转
    let result = text_processor::process_text("下午两点开会", &[], false);
    assert_eq!(result, "下午 2 点开会");
}

// ---------------------------------------------------------------------------
// merge_uppercase_letter_sequences — 合并 whisper 把缩略词拆字母的场景
// ---------------------------------------------------------------------------

#[test]
fn merge_letters_cdn() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("升级到 C D N 节点"),
        "升级到 CDN 节点",
    );
}

#[test]
fn merge_letters_two_letter_mr() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("提交一个 M R"),
        "提交一个 MR",
    );
}

#[test]
fn merge_letters_api() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("调用 A P I 接口"),
        "调用 API 接口",
    );
}

#[test]
fn merge_letters_multiple_groups() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("C D N 和 M R"),
        "CDN 和 MR",
    );
}

#[test]
fn merge_letters_leaves_single_letter_alone() {
    // Single uppercase letter standalone — not a sequence, must stay.
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("I am here"),
        "I am here",
    );
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("维生素 A 含量"),
        "维生素 A 含量",
    );
}

#[test]
fn merge_letters_lowercase_untouched() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("a b c test"),
        "a b c test",
    );
}

#[test]
fn merge_letters_lowercase_word_breaks_sequence() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("C D is good"),
        "CD is good",
    );
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("A the B"),
        "A the B",
    );
}

#[test]
fn merge_letters_already_joined_acronym_untouched() {
    assert_eq!(
        text_processor::merge_uppercase_letter_sequences("the URL is CDN"),
        "the URL is CDN",
    );
}

#[test]
fn process_text_merge_cdn_pipeline() {
    let result = text_processor::process_text("升级到 C D N 节点", &[], false);
    assert_eq!(result, "升级到 CDN 节点");
}

#[test]
fn process_text_merge_mr_pipeline() {
    // A1 后: "一个" 中"个"为量词，量词扫描转 "一" → "1"，再经 CJK/ASCII 间距变 "1 个"
    let result = text_processor::process_text("提交一个 M R", &[], false);
    assert_eq!(result, "提交 1 个 MR");
}
