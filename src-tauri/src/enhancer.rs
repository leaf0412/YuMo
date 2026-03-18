use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancerConfig {
    pub provider: LLMProvider,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

/// Build the system and user messages from templates.
/// If `user_template` contains `{{text}}`, replace it. Otherwise append text.
/// If vocabulary is non-empty, append it to the user message.
pub fn build_prompt(
    system: &str,
    user_template: &str,
    text: &str,
    vocabulary: &[String],
) -> (String, String) {
    let user_msg = if user_template.contains("{{text}}") {
        user_template.replace("{{text}}", text)
    } else {
        format!("{}\n\n{}", user_template, text)
    };

    let user_msg = if vocabulary.is_empty() {
        user_msg
    } else {
        format!(
            "{}\n\nCustom vocabulary (use these exact spellings): {}",
            user_msg,
            vocabulary.join(", ")
        )
    };

    (system.to_string(), user_msg)
}

/// Build OpenAI-compatible chat completion request body.
pub fn build_openai_request_body(model: &str, system: &str, user: &str) -> String {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    })
    .to_string()
}

/// Build Anthropic Messages API request body.
pub fn build_anthropic_request_body(model: &str, system: &str, user: &str) -> String {
    serde_json::json!({
        "model": model,
        "system": system,
        "messages": [
            {"role": "user", "content": user}
        ],
        "max_tokens": 4096
    })
    .to_string()
}

/// Parse OpenAI-compatible response to extract the assistant message.
pub fn parse_openai_response(body: &str) -> Result<String, AppError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AppError::Network(e.to_string()))?;

    if let Some(error) = v.get("error") {
        return Err(AppError::Network(
            error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string(),
        ));
    }

    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Network("Invalid response format".to_string()))
}

/// Parse Anthropic Messages API response.
pub fn parse_anthropic_response(body: &str) -> Result<String, AppError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AppError::Network(e.to_string()))?;

    if let Some(error) = v.get("error") {
        return Err(AppError::Network(
            error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string(),
        ));
    }

    v["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Network("Invalid response format".to_string()))
}

/// Call LLM API and return enhanced text.
pub async fn enhance(
    config: &EnhancerConfig,
    system_msg: &str,
    user_msg: &str,
) -> Result<String, AppError> {
    let client = reqwest::Client::new();

    match config.provider {
        LLMProvider::OpenAI | LLMProvider::Ollama => {
            let base = config
                .base_url
                .as_deref()
                .unwrap_or("https://api.openai.com/v1");
            let url = format!("{}/chat/completions", base);
            let body = build_openai_request_body(&config.model, system_msg, user_msg);

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?
                .text()
                .await?;

            parse_openai_response(&resp)
        }
        LLMProvider::Anthropic => {
            let url = "https://api.anthropic.com/v1/messages";
            let body = build_anthropic_request_body(&config.model, system_msg, user_msg);

            let resp = client
                .post(url)
                .header("x-api-key", &config.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?
                .text()
                .await?;

            parse_anthropic_response(&resp)
        }
    }
}
