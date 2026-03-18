use voiceink_tauri_lib::cloud;

#[test]
fn test_build_openai_multipart_request() {
    let config = cloud::CloudConfig {
        provider: cloud::CloudProvider::OpenAI,
        model: "whisper-1".into(),
        api_key: "sk-test".into(),
        base_url: None,
    };
    let req = cloud::build_request_info(&config, "en");
    assert!(req.url.contains("api.openai.com"));
    assert!(req.url.contains("transcriptions"));
    assert_eq!(req.auth_header, "Bearer sk-test");
}

#[test]
fn test_build_groq_request() {
    let config = cloud::CloudConfig {
        provider: cloud::CloudProvider::Groq,
        model: "whisper-large-v3".into(),
        api_key: "gsk-test".into(),
        base_url: None,
    };
    let req = cloud::build_request_info(&config, "en");
    assert!(req.url.contains("groq.com"));
    assert_eq!(req.auth_header, "Bearer gsk-test");
}

#[test]
fn test_build_deepgram_request() {
    let config = cloud::CloudConfig {
        provider: cloud::CloudProvider::Deepgram,
        model: "nova-2".into(),
        api_key: "dg-test".into(),
        base_url: None,
    };
    let req = cloud::build_request_info(&config, "en");
    assert!(req.url.contains("deepgram.com"));
    assert_eq!(req.auth_header, "Token dg-test");
}

#[test]
fn test_custom_base_url() {
    let config = cloud::CloudConfig {
        provider: cloud::CloudProvider::OpenAI,
        model: "whisper-1".into(),
        api_key: "sk-test".into(),
        base_url: Some("https://custom.api.com/v1".into()),
    };
    let req = cloud::build_request_info(&config, "en");
    assert!(req.url.starts_with("https://custom.api.com/v1"));
}

#[test]
fn test_parse_openai_transcription_response() {
    let body = r#"{"text":"Hello, world!"}"#;
    let result = cloud::parse_response(cloud::CloudProvider::OpenAI, body).unwrap();
    assert_eq!(result, "Hello, world!");
}

#[test]
fn test_parse_deepgram_response() {
    let body = r#"{"results":{"channels":[{"alternatives":[{"transcript":"Hello world"}]}]}}"#;
    let result = cloud::parse_response(cloud::CloudProvider::Deepgram, body).unwrap();
    assert_eq!(result, "Hello world");
}

#[test]
fn test_all_providers_listed() {
    let providers = cloud::available_providers();
    assert!(providers.len() >= 3);
    assert!(providers.iter().any(|p| p.id == "openai"));
    assert!(providers.iter().any(|p| p.id == "groq"));
    assert!(providers.iter().any(|p| p.id == "deepgram"));
}
