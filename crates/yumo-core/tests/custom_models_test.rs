use yumo_core::custom_models::{parse_spec_from_str, CustomModelSpec};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn parses_minimal_valid_yaml() {
    let yaml = r#"
schema_version: 1
id: test-asr
name: Test ASR
size_mb: 100
languages:
  zh: 中文
speed: 5
accuracy: 5
python_module: fake_asr_pkg
load:
  function: fake_asr_pkg.load_asr
  kwargs: {}
"#;
    let spec = parse_spec_from_str(yaml, PathBuf::from("/tmp/test.yaml"))
        .expect("should parse");

    assert_eq!(spec.schema_version, 1);
    assert_eq!(spec.id, "test-asr");
    assert_eq!(spec.name, "Test ASR");
    assert_eq!(spec.size_mb, 100);
    assert_eq!(spec.speed, 5);
    assert_eq!(spec.accuracy, 5);
    assert_eq!(spec.python_module, "fake_asr_pkg");
    assert_eq!(spec.load.function, "fake_asr_pkg.load_asr");
    assert!(spec.load.kwargs.is_empty());

    // Defaults applied
    assert_eq!(spec.transcribe_method, "transcribe");
    assert_eq!(spec.language_param, "language");
    assert_eq!(spec.recommended, false);
    assert_eq!(spec.pip_packages, vec!["fake_asr_pkg".to_string()]);
    assert!(spec.download.is_none());
    assert!(spec.description.is_none());

    let langs: HashMap<String, String> = [("zh".to_string(), "中文".to_string())].into();
    assert_eq!(spec.languages, langs);
}
