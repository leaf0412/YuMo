use yumo_lib::enhancer;

#[test]
fn test_build_prompt_with_template() {
    let system = "You clean up transcribed text.";
    let user_template = "Clean this: {{text}}";
    let vocabulary = vec!["Kubernetes".to_string(), "TypeScript".to_string()];

    let (sys, user) = enhancer::build_prompt(system, user_template, "hello wrold", &vocabulary);
    assert_eq!(sys, "You clean up transcribed text.");
    assert!(user.contains("hello wrold"));
    assert!(user.contains("Kubernetes"));
    assert!(user.contains("TypeScript"));
}

#[test]
fn test_build_prompt_no_vocabulary() {
    let (_, user) = enhancer::build_prompt("sys", "Fix: {{text}}", "test input", &[]);
    assert_eq!(user, "Fix: test input");
}

#[test]
fn test_build_prompt_no_template_placeholder() {
    let (_, user) = enhancer::build_prompt("sys", "Just do it", "test input", &[]);
    assert!(user.contains("test input"));
}

#[test]
fn test_build_openai_request_body() {
    let body = enhancer::build_openai_request_body("gpt-4o-mini", "system msg", "user msg");
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["model"], "gpt-4o-mini");
    assert_eq!(parsed["messages"][0]["role"], "system");
    assert_eq!(parsed["messages"][0]["content"], "system msg");
    assert_eq!(parsed["messages"][1]["role"], "user");
    assert_eq!(parsed["messages"][1]["content"], "user msg");
}

#[test]
fn test_build_anthropic_request_body() {
    let body =
        enhancer::build_anthropic_request_body("claude-sonnet-4-20250514", "system msg", "user msg");
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["model"], "claude-sonnet-4-20250514");
    assert_eq!(parsed["system"], "system msg");
    assert_eq!(parsed["messages"][0]["role"], "user");
    assert_eq!(parsed["messages"][0]["content"], "user msg");
    assert!(parsed["max_tokens"].as_i64().unwrap() > 0);
}

#[test]
fn test_parse_openai_response() {
    let response = r#"{"choices":[{"message":{"content":"Fixed text here"}}]}"#;
    let result = enhancer::parse_openai_response(response).unwrap();
    assert_eq!(result, "Fixed text here");
}

#[test]
fn test_parse_anthropic_response() {
    let response = r#"{"content":[{"type":"text","text":"Fixed text here"}]}"#;
    let result = enhancer::parse_anthropic_response(response).unwrap();
    assert_eq!(result, "Fixed text here");
}

#[test]
fn test_parse_openai_response_error() {
    let response = r#"{"error":{"message":"Invalid API key"}}"#;
    let result = enhancer::parse_openai_response(response);
    assert!(result.is_err());
}
