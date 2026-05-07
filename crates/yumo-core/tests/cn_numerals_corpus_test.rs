//! 语料库回归测试 - 从 tests/corpus/cn_numerals.tsv 读取真实样本
//!
//! 失败时一次性 panic 列出所有失败 case，便于 diff 对比。
//! 后续用户从生产日志脱敏导出，按格式追加即可，CI 自动跑。

use std::fs;
use std::path::PathBuf;
use yumo_core::text_processor;

#[test]
fn cn_numerals_corpus_regression() {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join("cn_numerals.tsv");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read corpus failed at {:?}: {}", path, e));

    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut passed = 0usize;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            failures.push(format!(
                "line {}: malformed (expected at least 2 TAB-separated fields): {:?}",
                line_no, line
            ));
            continue;
        }
        let input = parts[0];
        let expected = parts[1];
        total += 1;
        let actual = text_processor::convert_cn_numerals(input);
        if actual == expected {
            passed += 1;
        } else {
            failures.push(format!(
                "line {} FAIL\n  input    = {:?}\n  expected = {:?}\n  actual   = {:?}",
                line_no, input, expected, actual
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n=== cn_numerals corpus regression: {}/{} passed, {} failed ===\n\n{}\n",
            passed,
            total,
            failures.len(),
            failures.join("\n\n")
        );
    }

    eprintln!("[corpus] {}/{} passed", passed, total);
}
