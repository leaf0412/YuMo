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
// chinese_numerals_to_arabic
// ---------------------------------------------------------------------------

#[test]
fn cn_num_skips_single_chars() {
    // Single CJK numeral chars must NOT convert — guards against false-positives
    // on idioms like 一些 / 二话不说 / 三明治 / 九点 / 万一.
    assert_eq!(text_processor::chinese_numerals_to_arabic("一些事"), "一些事");
    assert_eq!(text_processor::chinese_numerals_to_arabic("一下子"), "一下子");
    assert_eq!(text_processor::chinese_numerals_to_arabic("一定可以"), "一定可以");
    assert_eq!(text_processor::chinese_numerals_to_arabic("二话不说"), "二话不说");
    assert_eq!(text_processor::chinese_numerals_to_arabic("三明治"), "三明治");
    assert_eq!(text_processor::chinese_numerals_to_arabic("九点开会"), "九点开会");
    assert_eq!(text_processor::chinese_numerals_to_arabic("万一发生"), "万一发生");
    assert_eq!(text_processor::chinese_numerals_to_arabic("二月一号"), "二月一号");
    assert_eq!(text_processor::chinese_numerals_to_arabic("十块钱"), "十块钱");
}

#[test]
fn cn_num_simple_units() {
    assert_eq!(text_processor::chinese_numerals_to_arabic("一百块"), "100块");
    assert_eq!(text_processor::chinese_numerals_to_arabic("二十"), "20");
    assert_eq!(text_processor::chinese_numerals_to_arabic("两百"), "200");
    assert_eq!(text_processor::chinese_numerals_to_arabic("三千"), "3000");
    assert_eq!(text_processor::chinese_numerals_to_arabic("一万"), "10000");
    assert_eq!(text_processor::chinese_numerals_to_arabic("二十五"), "25");
    assert_eq!(text_processor::chinese_numerals_to_arabic("三百二十一"), "321");
}

#[test]
fn cn_num_positional() {
    // Pure-digit tokens: positional concatenation.
    assert_eq!(text_processor::chinese_numerals_to_arabic("二〇二六年"), "2026年");
    assert_eq!(text_processor::chinese_numerals_to_arabic("一九八四"), "1984");
    assert_eq!(text_processor::chinese_numerals_to_arabic("二三"), "23");
}

#[test]
fn cn_num_complex() {
    assert_eq!(text_processor::chinese_numerals_to_arabic("一万两千三百四十五"), "12345");
    assert_eq!(text_processor::chinese_numerals_to_arabic("一千零五"), "1005");
    assert_eq!(text_processor::chinese_numerals_to_arabic("两亿三千万"), "230000000");
    assert_eq!(text_processor::chinese_numerals_to_arabic("十万"), "100000");
}

#[test]
fn cn_num_mixed_in_sentence() {
    assert_eq!(
        text_processor::chinese_numerals_to_arabic("我有三百二十一块钱，他有一万"),
        "我有321块钱，他有10000",
    );
    assert_eq!(
        text_processor::chinese_numerals_to_arabic("生于二〇〇一年，今年二十五岁"),
        "生于2001年，今年25岁",
    );
}

#[test]
fn cn_num_no_change_on_non_chinese() {
    assert_eq!(text_processor::chinese_numerals_to_arabic("hello"), "hello");
    assert_eq!(text_processor::chinese_numerals_to_arabic("1234"), "1234");
    assert_eq!(text_processor::chinese_numerals_to_arabic(""), "");
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
