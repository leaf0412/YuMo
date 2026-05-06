use yumo_core::custom_models::{parse_spec_from_str, CustomModelSpec};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use yumo_core::custom_models::validate_spec;

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

#[test]
fn parses_full_yaml_with_function_download() {
    let yaml = r#"
schema_version: 1
id: custom-mimo-int4
name: MiMo INT4
description: Xiaomi MiMo V2.5 ASR INT4
size_mb: 8000
languages:
  zh: 中文
  en: English
speed: 7
accuracy: 9
recommended: false
python_module: mimo_mlx
pip_packages:
  - mimo_mlx>=0.1.0
download:
  function: mimo_mlx.download_models
  kwargs:
    precision: int4
    audio_tokenizer_repo: XiaomiMiMo/MiMo-Audio-Tokenizer
  returns: tuple
  path_names: [asr_dir, tokenizer_dir]
load:
  function: mimo_mlx.load_asr
  kwargs:
    precision: int4
    audio_tokenizer_dir: "{paths.tokenizer_dir}"
transcribe_method: transcribe
language_param: language
"#;
    let spec = parse_spec_from_str(yaml, PathBuf::from("/tmp/mimo.yaml")).unwrap();

    assert_eq!(spec.id, "custom-mimo-int4");
    assert_eq!(spec.description.as_deref(), Some("Xiaomi MiMo V2.5 ASR INT4"));
    assert_eq!(spec.pip_packages, vec!["mimo_mlx>=0.1.0".to_string()]);

    match spec.download.unwrap() {
        yumo_core::custom_models::DownloadSpec::Function { function, kwargs, returns, path_names } => {
            assert_eq!(function, "mimo_mlx.download_models");
            assert_eq!(kwargs.get("precision").unwrap().as_str(), Some("int4"));
            assert!(matches!(returns, yumo_core::custom_models::DownloadReturnKind::Tuple));
            assert_eq!(path_names, vec!["asr_dir".to_string(), "tokenizer_dir".to_string()]);
        }
        _ => panic!("expected Function variant"),
    }
}

#[test]
fn parses_yaml_with_hf_repos_download() {
    let yaml = r#"
schema_version: 1
id: custom-simple
name: Simple
size_mb: 500
languages:
  en: English
speed: 5
accuracy: 5
python_module: some_pkg
download:
  hf_repos:
    - foo/bar
    - foo/baz
  paths:
    asr_dir: "{repo_dirs[0]}"
    tok_dir: "{repo_dirs[1]}"
load:
  function: some_pkg.load
  kwargs: {}
"#;
    let spec = parse_spec_from_str(yaml, PathBuf::from("/tmp/x.yaml")).unwrap();

    match spec.download.unwrap() {
        yumo_core::custom_models::DownloadSpec::HfRepos { hf_repos, paths } => {
            assert_eq!(hf_repos, vec!["foo/bar".to_string(), "foo/baz".to_string()]);
            assert_eq!(paths.get("asr_dir").unwrap(), "{repo_dirs[0]}");
        }
        _ => panic!("expected HfRepos variant"),
    }
}

fn make_minimal_spec(id: &str) -> CustomModelSpec {
    let yaml = format!(r#"
schema_version: 1
id: {}
name: Test
size_mb: 1
languages:
  zh: 中文
speed: 5
accuracy: 5
python_module: pkg
load:
  function: pkg.load
  kwargs: {{}}
"#, id);
    parse_spec_from_str(&yaml, PathBuf::from("/tmp/t.yaml")).unwrap()
}

#[test]
fn validate_rejects_id_collision_with_builtin() {
    let spec = make_minimal_spec("ggml-tiny");
    let builtin: HashSet<String> = ["ggml-tiny".to_string()].into_iter().collect();
    let err = validate_spec(&spec, &builtin).unwrap_err();
    assert!(err.to_string().contains("ggml-tiny"));
    assert!(err.to_string().to_lowercase().contains("collide") || err.to_string().contains("已存在"));
}

#[test]
fn validate_rejects_speed_out_of_range() {
    let mut spec = make_minimal_spec("ok");
    spec.speed = 11;
    let err = validate_spec(&spec, &HashSet::new()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("speed"));
}

#[test]
fn validate_rejects_accuracy_out_of_range() {
    let mut spec = make_minimal_spec("ok");
    spec.accuracy = 0;
    let err = validate_spec(&spec, &HashSet::new()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("accuracy"));
}

#[test]
fn validate_rejects_empty_languages() {
    let mut spec = make_minimal_spec("ok");
    spec.languages.clear();
    let err = validate_spec(&spec, &HashSet::new()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("languages"));
}

#[test]
fn validate_rejects_unsupported_schema_version() {
    let mut spec = make_minimal_spec("ok");
    spec.schema_version = 99;
    let err = validate_spec(&spec, &HashSet::new()).unwrap_err();
    assert!(err.to_string().contains("schema_version"));
}

#[test]
fn validate_passes_for_valid_spec() {
    let spec = make_minimal_spec("custom-ok");
    validate_spec(&spec, &HashSet::new()).unwrap();
}

use yumo_core::custom_models::{scan_custom_models, ScanResult};
use yumo_core::custom_models::spec_to_model_info;
use yumo_core::transcriber::ModelProvider;

#[test]
fn spec_to_model_info_uses_custom_provider_and_path_in_repo() {
    let spec = make_minimal_spec("test-id");
    let info = spec_to_model_info(&spec);

    assert_eq!(info.id, "test-id");
    assert_eq!(info.name, "Test");
    assert_eq!(info.size_mb, 1);
    assert!(matches!(info.provider, ModelProvider::Custom));
    assert_eq!(info.model_repo.as_deref(), Some(spec.source_path.to_str().unwrap()));
    assert_eq!(info.is_downloaded, false);
    assert_eq!(info.languages, vec!["zh".to_string()]);
}

#[test]
fn spec_to_model_info_multilang_uses_multi_marker() {
    let yaml = r#"
schema_version: 1
id: multi
name: Multi
size_mb: 1
languages:
  zh: 中文
  en: English
speed: 5
accuracy: 5
python_module: x
load:
  function: x.load
  kwargs: {}
"#;
    let spec = parse_spec_from_str(yaml, PathBuf::from("/tmp/t.yaml")).unwrap();
    let info = spec_to_model_info(&spec);
    assert_eq!(info.languages, vec!["multi".to_string()]);
}

#[test]
fn scan_returns_empty_for_nonexistent_dir() {
    let results = scan_custom_models(&PathBuf::from("/nonexistent/path/xyz"));
    assert!(results.is_empty());
}

#[test]
fn scan_returns_empty_for_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let results = scan_custom_models(dir.path());
    assert!(results.is_empty());
}

#[test]
fn scan_isolates_invalid_files_from_valid_ones() {
    let dir = tempfile::tempdir().unwrap();
    let valid = r#"
schema_version: 1
id: ok
name: OK
size_mb: 1
languages: { en: English }
speed: 5
accuracy: 5
python_module: x
load:
  function: x.load
  kwargs: {}
"#;
    std::fs::write(dir.path().join("good.yaml"), valid).unwrap();
    std::fs::write(dir.path().join("broken.yaml"), "not: : valid: yaml::").unwrap();
    std::fs::write(dir.path().join("ignored.txt"), "should be skipped").unwrap();

    let mut results = scan_custom_models(dir.path());
    results.sort_by_key(|r| match r {
        ScanResult::Ok(s) => s.source_path.file_name().unwrap().to_string_lossy().into_owned(),
        ScanResult::Err { path, .. } => path.file_name().unwrap().to_string_lossy().into_owned(),
    });

    assert_eq!(results.len(), 2);
    assert!(matches!(results[0], ScanResult::Err { .. }), "results[0] should be Err");
    assert!(matches!(results[1], ScanResult::Ok(_)), "results[1] should be Ok");
}
