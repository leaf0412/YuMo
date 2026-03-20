use yumo_lib::transcriber;

#[test]
fn test_predefined_models_list() {
    let models = transcriber::predefined_models();
    assert!(!models.is_empty());

    let base_en = models.iter().find(|m| m.id == "ggml-base.en");
    assert!(
        base_en.is_some(),
        "Should have ggml-base.en in predefined list"
    );
    let base = base_en.unwrap();
    assert!(base.size_mb > 0);
    assert!(!base.download_url.is_empty());
    assert!(base.download_url.contains("huggingface.co"));
}

#[test]
fn test_model_path_resolution() {
    let models_dir = std::path::PathBuf::from("/tmp/voiceink-test-models");
    let path = transcriber::model_path(&models_dir, "ggml-base.en");
    assert_eq!(path, models_dir.join("ggml-base.en.bin"));
}

#[test]
fn test_check_downloaded_models() {
    let tmp = tempfile::TempDir::new().unwrap();
    let models_dir = tmp.path().to_path_buf();

    // No models downloaded yet
    let models = transcriber::check_downloaded_models(&models_dir);
    assert!(models.iter().all(|m| !m.is_downloaded));

    // Create a fake model file
    std::fs::write(models_dir.join("ggml-base.en.bin"), b"fake").unwrap();
    let models = transcriber::check_downloaded_models(&models_dir);
    let base = models.iter().find(|m| m.id == "ggml-base.en").unwrap();
    assert!(base.is_downloaded);
}

#[test]
fn test_format_text_capitalize() {
    let result = transcriber::format_text("hello world. this is a test.", true, false);
    assert_eq!(result, "Hello world. This is a test.");
}

#[test]
fn test_format_text_no_options() {
    let result = transcriber::format_text("hello world", false, false);
    assert_eq!(result, "hello world");
}
