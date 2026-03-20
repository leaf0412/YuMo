use yumo_lib::transcriber::{
    self, ModelInfo, ModelProvider, ModelFilter,
};

// ---------------------------------------------------------------------------
// ModelProvider categorization
// ---------------------------------------------------------------------------

#[test]
fn test_provider_is_local() {
    assert!(ModelProvider::Local.is_local());
    assert!(ModelProvider::MlxWhisper.is_local());
    assert!(ModelProvider::MlxFunASR.is_local());
    assert!(!ModelProvider::Groq.is_local());
    assert!(!ModelProvider::Deepgram.is_local());
}

#[test]
fn test_provider_is_cloud() {
    assert!(ModelProvider::Groq.is_cloud());
    assert!(ModelProvider::Deepgram.is_cloud());
    assert!(ModelProvider::ElevenLabs.is_cloud());
    assert!(ModelProvider::Mistral.is_cloud());
    assert!(ModelProvider::Gemini.is_cloud());
    assert!(ModelProvider::Soniox.is_cloud());
    assert!(!ModelProvider::Local.is_cloud());
    assert!(!ModelProvider::MlxFunASR.is_cloud());
}

#[test]
fn test_provider_needs_daemon() {
    assert!(ModelProvider::MlxFunASR.needs_daemon());
    assert!(ModelProvider::MlxWhisper.needs_daemon());
    assert!(!ModelProvider::Local.needs_daemon());
    assert!(!ModelProvider::Groq.needs_daemon());
}

// ---------------------------------------------------------------------------
// ModelFilter
// ---------------------------------------------------------------------------

#[test]
fn test_filter_recommended_non_empty() {
    let all = transcriber::all_predefined_models();
    let recommended: Vec<_> = all.iter().filter(|m| m.is_recommended).collect();
    assert!(!recommended.is_empty(), "should have recommended models");
}

#[test]
fn test_filter_local_only() {
    let all = transcriber::all_predefined_models();
    let local: Vec<_> = all.iter().filter(|m| m.provider.is_local()).collect();
    assert!(local.len() >= 4, "should have at least 4 local models, got {}", local.len());
}

#[test]
fn test_filter_cloud_only() {
    let all = transcriber::all_predefined_models();
    let cloud: Vec<_> = all.iter().filter(|m| m.provider.is_cloud()).collect();
    assert!(!cloud.is_empty(), "should have cloud models");
}

// ---------------------------------------------------------------------------
// Model metadata
// ---------------------------------------------------------------------------

#[test]
fn test_model_has_speed_and_accuracy() {
    let all = transcriber::all_predefined_models();
    let base_en = all.iter().find(|m| m.id == "ggml-base.en").unwrap();
    assert!(base_en.speed > 0, "speed should be set");
    assert!(base_en.accuracy > 0, "accuracy should be set");
}

#[test]
fn test_model_supported_languages_map() {
    let all = transcriber::all_predefined_models();
    let base_en = all.iter().find(|m| m.id == "ggml-base.en").unwrap();
    assert!(base_en.supported_languages.contains_key("en"));
    assert_eq!(base_en.supported_languages.get("en").unwrap(), "English");
}

#[test]
fn test_multilingual_model_has_multiple_languages() {
    let all = transcriber::all_predefined_models();
    let large = all.iter().find(|m| m.id == "ggml-large-v3").unwrap();
    assert!(large.supported_languages.len() > 1, "large-v3 should be multilingual");
    assert!(large.supported_languages.contains_key("zh"));
    assert!(large.supported_languages.contains_key("en"));
}

// ---------------------------------------------------------------------------
// MLX Whisper models exist
// ---------------------------------------------------------------------------

#[test]
fn test_mlx_whisper_models_defined() {
    let all = transcriber::all_predefined_models();
    let mlx_whisper: Vec<_> = all.iter()
        .filter(|m| matches!(m.provider, ModelProvider::MlxWhisper))
        .collect();
    assert!(mlx_whisper.len() >= 3, "should have at least 3 MLX Whisper models, got {}", mlx_whisper.len());

    let names: Vec<&str> = mlx_whisper.iter().map(|m| m.id.as_str()).collect();
    assert!(names.contains(&"mlx-whisper-large-v3"), "missing large-v3");
    assert!(names.contains(&"mlx-whisper-distil-large-v3"), "missing distil-large-v3");
    assert!(names.contains(&"mlx-whisper-small"), "missing small");
}

#[test]
fn test_mlx_whisper_has_model_repo() {
    let all = transcriber::all_predefined_models();
    let mlx = all.iter().find(|m| m.id == "mlx-whisper-large-v3").unwrap();
    assert!(mlx.model_repo.is_some());
    assert!(mlx.model_repo.as_ref().unwrap().contains("mlx-community"));
}

// ---------------------------------------------------------------------------
// Cloud models
// ---------------------------------------------------------------------------

#[test]
fn test_groq_model_defined() {
    let all = transcriber::all_predefined_models();
    let groq: Vec<_> = all.iter()
        .filter(|m| matches!(m.provider, ModelProvider::Groq))
        .collect();
    assert!(!groq.is_empty(), "should have Groq models");
}

#[test]
fn test_cloud_models_have_no_download_url() {
    let all = transcriber::all_predefined_models();
    for m in all.iter().filter(|m| m.provider.is_cloud()) {
        assert!(m.download_url.is_empty(), "cloud model {} should not have download_url", m.id);
    }
}

// ---------------------------------------------------------------------------
// Backward compatibility
// ---------------------------------------------------------------------------

#[test]
fn test_predefined_models_still_works() {
    // Old function should still return local whisper models
    let models = transcriber::predefined_models();
    assert!(models.iter().any(|m| m.id == "ggml-base.en"));
}

#[test]
fn test_predefined_mlx_models_still_works() {
    let models = transcriber::predefined_mlx_models();
    assert!(models.iter().any(|m| m.id == "mlx-funasr-nano-8bit"));
}
