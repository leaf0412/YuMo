//! 自 2026-05 中文数字识别重构后:
//! - "独立中文数字串无量词锚点"不再转换（强证据原则）
//!   例: "二十" → "二十", 但 "二十块" → "20块"
//! - 单字数字+量词转换（A1 决策）
//!   例: "三个" → "3个", "九点" → "9点"
//! - 部分场景 (一点 / 三点 / 二点零) 因量词扫描接管，跟旧 ≥3 段版本号约束行为不同
//!   设计文档: _docs/specs/2026-05-07-cn-numerals-redesign-design.md

use yumo_core::text_processor;
use yumo_core::text_processor::ProcessOptions;

// Test helper: build a ProcessOptions with every step enabled. Useful for tests
// that exercise the full historical pipeline (cn_numerals, capitalize, ...).
fn opts_all_on() -> ProcessOptions {
    ProcessOptions {
        auto_capitalize: true,
        append_period: true,
        convert_cn_numerals: true,
        use_builtin_dictionary: true,
    }
}

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
    // append_period: true → 句中 ". " 保留, 末尾 "." 已存在不补
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        &ProcessOptions {
            auto_capitalize: true,
            append_period: true,
            ..Default::default()
        },
    );
    assert_eq!(result, "I deploy to Kubernetes. It works.");
}

#[test]
fn test_process_text_no_capitalize() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    // append_period: false (default) → 末尾 "." 主动剥掉, 句中 ". " 不动
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        &ProcessOptions::default(),
    );
    assert_eq!(result, "i deploy to Kubernetes. it works");
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
    let result = text_processor::process_text(
        "我有一百块Hello",
        &[],
        &ProcessOptions {
            convert_cn_numerals: true,
            ..Default::default()
        },
    );
    assert_eq!(result, "我有 100 块 Hello");
}

#[test]
fn process_text_idiom_safe() {
    // Single-char numerals in idioms must not be touched even with formatting on.
    let result = text_processor::process_text(
        "一些事情发生了",
        &[],
        &ProcessOptions {
            convert_cn_numerals: true,
            ..Default::default()
        },
    );
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
    let result = text_processor::process_text(
        "可以发布零点六点零的版本",
        &[],
        &ProcessOptions {
            convert_cn_numerals: true,
            ..Default::default()
        },
    );
    assert_eq!(result, "可以发布 0.6.0 的版本");
}

#[test]
fn process_text_version_does_not_touch_time() {
    // A1 后行为变化：原期望 "下午两点开会"，新版本量词扫描会转
    let result = text_processor::process_text(
        "下午两点开会",
        &[],
        &ProcessOptions {
            convert_cn_numerals: true,
            ..Default::default()
        },
    );
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
    let result = text_processor::process_text(
        "升级到 C D N 节点",
        &[],
        &ProcessOptions::default(),
    );
    assert_eq!(result, "升级到 CDN 节点");
}

#[test]
fn process_text_merge_mr_pipeline() {
    // A1 后: "一个" 中"个"为量词，量词扫描转 "一" → "1"，再经 CJK/ASCII 间距变 "1 个"
    let result = text_processor::process_text(
        "提交一个 M R",
        &[],
        &ProcessOptions {
            convert_cn_numerals: true,
            ..Default::default()
        },
    );
    assert_eq!(result, "提交 1 个 MR");
}

// ---------------------------------------------------------------------------
// append_terminal_period — 句末追加句号（locale-自适应）
// ---------------------------------------------------------------------------

#[test]
fn period_chinese_end() {
    assert_eq!(text_processor::append_terminal_period("今天天气好"), "今天天气好。");
    assert_eq!(text_processor::append_terminal_period("好的"), "好的。");
}

#[test]
fn period_english_end() {
    assert_eq!(text_processor::append_terminal_period("the weather is good"), "the weather is good.");
    assert_eq!(text_processor::append_terminal_period("ok"), "ok.");
}

#[test]
fn period_already_terminal_skipped() {
    // ASCII terminators
    assert_eq!(text_processor::append_terminal_period("done."), "done.");
    assert_eq!(text_processor::append_terminal_period("really?"), "really?");
    assert_eq!(text_processor::append_terminal_period("wow!"), "wow!");
    // Chinese terminators
    assert_eq!(text_processor::append_terminal_period("好了。"), "好了。");
    assert_eq!(text_processor::append_terminal_period("真的吗？"), "真的吗？");
    assert_eq!(text_processor::append_terminal_period("绝了！"), "绝了！");
    // Ellipsis
    assert_eq!(text_processor::append_terminal_period("等等…"), "等等…");
}

#[test]
fn period_empty_or_whitespace_only_returns_original() {
    assert_eq!(text_processor::append_terminal_period(""), "");
    assert_eq!(text_processor::append_terminal_period("   "), "   ");
    assert_eq!(text_processor::append_terminal_period("\n\n"), "\n\n");
}

#[test]
fn period_strips_trailing_whitespace_before_appending() {
    assert_eq!(text_processor::append_terminal_period("hello  "), "hello.");
    assert_eq!(text_processor::append_terminal_period("好的 \n"), "好的。");
}

#[test]
fn period_picks_char_by_last_non_space_char() {
    // 末字符是中文 → 中文句号
    assert_eq!(text_processor::append_terminal_period("API 调用"), "API 调用。");
    // 末字符是 ASCII 字母 → ASCII 句号
    assert_eq!(text_processor::append_terminal_period("调用 API"), "调用 API.");
    // 末字符是 ASCII 数字 → ASCII 句号
    assert_eq!(text_processor::append_terminal_period("总共 100"), "总共 100.");
}

// ---------------------------------------------------------------------------
// strip_terminal_period — append_period=false 时主动剥末尾句号
// 只剥「。/.」, 「?!？！…⋯」一律保留 (疑问/感叹/省略号有表达力)
// ---------------------------------------------------------------------------

#[test]
fn strip_period_chinese_end() {
    assert_eq!(text_processor::strip_terminal_period("你好。"), "你好");
    assert_eq!(text_processor::strip_terminal_period("不输出句号的。"), "不输出句号的");
}

#[test]
fn strip_period_ascii_end() {
    assert_eq!(text_processor::strip_terminal_period("Skills."), "Skills");
    assert_eq!(text_processor::strip_terminal_period("ok."), "ok");
}

#[test]
fn strip_period_preserves_question_exclamation() {
    assert_eq!(text_processor::strip_terminal_period("真的吗？"), "真的吗？");
    assert_eq!(text_processor::strip_terminal_period("绝了！"), "绝了！");
    assert_eq!(text_processor::strip_terminal_period("really?"), "really?");
    assert_eq!(text_processor::strip_terminal_period("wow!"), "wow!");
}

#[test]
fn strip_period_preserves_ellipsis() {
    // CJK 省略号
    assert_eq!(text_processor::strip_terminal_period("等等…"), "等等…");
    assert_eq!(text_processor::strip_terminal_period("等等⋯"), "等等⋯");
    // ASCII "..." (≥3 dots) 视为省略号, 保留
    assert_eq!(text_processor::strip_terminal_period("Hello..."), "Hello...");
    assert_eq!(text_processor::strip_terminal_period("etc...."), "etc....");
}

#[test]
fn strip_period_no_terminal_punct_returns_original() {
    assert_eq!(text_processor::strip_terminal_period("今天天气好"), "今天天气好");
    assert_eq!(text_processor::strip_terminal_period("hello"), "hello");
}

#[test]
fn strip_period_multiple_trailing_periods() {
    // 连续 "。。" 全部剥掉, 不留半截
    assert_eq!(text_processor::strip_terminal_period("你好。。"), "你好");
    // "Hello.." (2 dots, 非省略号) 全部剥掉
    assert_eq!(text_processor::strip_terminal_period("Hello.."), "Hello");
}

#[test]
fn strip_period_handles_trailing_whitespace() {
    // 末尾空白先 trim, 然后剥句号; 与 append_terminal_period 对称
    assert_eq!(text_processor::strip_terminal_period("你好。 "), "你好");
    assert_eq!(text_processor::strip_terminal_period("Skills. \n"), "Skills");
}

#[test]
fn strip_period_empty_or_whitespace_only_returns_original() {
    assert_eq!(text_processor::strip_terminal_period(""), "");
    assert_eq!(text_processor::strip_terminal_period("   "), "   ");
    assert_eq!(text_processor::strip_terminal_period("\n\n"), "\n\n");
}

// ---------------------------------------------------------------------------
// apply_builtin_dict — 内置错别字词典
// ---------------------------------------------------------------------------

#[test]
fn builtin_dict_replaces_chinese_typo_inline() {
    // 嵌入式 CJK 替换：百渡 → 百度（即使两侧都被其它中文字包围）
    assert_eq!(text_processor::apply_builtin_dict("我用百渡搜索"), "我用百度搜索");
}

#[test]
fn builtin_dict_replaces_ascii_brand_case_insensitive() {
    assert_eq!(text_processor::apply_builtin_dict("clone from github"), "clone from GitHub");
    assert_eq!(text_processor::apply_builtin_dict("Github page"), "GitHub page");
}

#[test]
fn builtin_dict_ascii_respects_word_boundary() {
    // "githubber" 不应被替换 — \b 限制
    let result = text_processor::apply_builtin_dict("githubber is fake");
    assert_eq!(result, "githubber is fake");
}

#[test]
fn builtin_dict_no_op_when_no_match() {
    assert_eq!(text_processor::apply_builtin_dict("一切正常"), "一切正常");
    assert_eq!(text_processor::apply_builtin_dict(""), "");
}

#[test]
fn builtin_dict_multiple_entries_in_one_text() {
    let result = text_processor::apply_builtin_dict("找腾迅或者百渡");
    assert_eq!(result, "找腾讯或者百度");
}

// ---------------------------------------------------------------------------
// ProcessOptions — toggle 各组合的 process_text 行为
// ---------------------------------------------------------------------------

#[test]
fn process_text_default_options_skip_optional_steps() {
    // 默认 ProcessOptions: 全 false。仅 always-on 步骤生效（replace/letter_merge/cjk_spacing）。
    let result = text_processor::process_text("我有一百块Hello", &[], &ProcessOptions::default());
    // cn_numerals 关 → "一百块" 不变；只有 CJK/ASCII 空格生效
    assert_eq!(result, "我有一百块 Hello");
}

#[test]
fn process_text_period_toggle_appends_only_when_enabled() {
    let on = ProcessOptions { append_period: true, ..Default::default() };
    let result_on = text_processor::process_text("今天天气好", &[], &on);
    assert_eq!(result_on, "今天天气好。");
    let result_off = text_processor::process_text("今天天气好", &[], &ProcessOptions::default());
    assert_eq!(result_off, "今天天气好");
}

#[test]
fn process_text_period_toggle_off_strips_trailing_period() {
    // append_period=false 不仅"不补"句号, 还要主动剥掉模型 (Whisper) 自带的尾句号,
    // 否则用户视角看不出开关有任何影响。?!? ! 等保留。
    let off = ProcessOptions::default();
    assert_eq!(text_processor::process_text("你好。", &[], &off), "你好");
    assert_eq!(text_processor::process_text("Skills.", &[], &off), "Skills");
    assert_eq!(text_processor::process_text("真的吗？", &[], &off), "真的吗？");
    assert_eq!(text_processor::process_text("绝了！", &[], &off), "绝了！");
    // 省略号保留
    assert_eq!(text_processor::process_text("等等…", &[], &off), "等等…");
}

#[test]
fn process_text_cn_numerals_toggle_off_keeps_chinese_numerals() {
    let result = text_processor::process_text(
        "我有一百块",
        &[],
        &ProcessOptions::default(),
    );
    assert_eq!(result, "我有一百块");
}

#[test]
fn process_text_cn_numerals_toggle_on_converts() {
    let result = text_processor::process_text(
        "我有一百块",
        &[],
        &ProcessOptions { convert_cn_numerals: true, ..Default::default() },
    );
    assert_eq!(result, "我有 100 块");
}

#[test]
fn process_text_builtin_dict_toggle() {
    // ON: 内置词典生效
    let result_on = text_processor::process_text(
        "找腾迅客服",
        &[],
        &ProcessOptions { use_builtin_dictionary: true, ..Default::default() },
    );
    assert_eq!(result_on, "找腾讯客服");
    // OFF: 不替换
    let result_off = text_processor::process_text(
        "找腾迅客服",
        &[],
        &ProcessOptions::default(),
    );
    assert_eq!(result_off, "找腾迅客服");
}

#[test]
fn process_text_user_replacement_runs_before_builtin_dict() {
    // 用户规则: github → MyOrg。内置规则: github → GitHub。用户规则先跑，结果中 "github" 已不存在，内置不再触发。
    let user = vec![("github".to_string(), "MyOrg".to_string())];
    let result = text_processor::process_text(
        "clone from github",
        &user,
        &ProcessOptions { use_builtin_dictionary: true, ..Default::default() },
    );
    assert_eq!(result, "clone from MyOrg");
}

#[test]
fn process_text_all_toggles_on_pipeline() {
    // 用 opts_all_on() 跑一遍混合场景：cn_numerals + capitalize + period + builtin_dict 都激活
    let result = text_processor::process_text(
        "i deploy to github with 一百块",
        &[],
        &opts_all_on(),
    );
    // builtin: github → GitHub
    // cn_numerals: 一百块 → 100 块
    // capitalize: I, ... (only句首/.?! 后)
    // period: ends with "块" (CJK) → append "。"
    assert_eq!(result, "I deploy to GitHub with 100 块。");
}
