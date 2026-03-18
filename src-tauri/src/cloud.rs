use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    OpenAI,
    Groq,
    Deepgram,
    ElevenLabs,
    Gemini,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub provider: CloudProvider,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub default_model: String,
}

pub struct RequestInfo {
    pub url: String,
    pub auth_header: String,
}

pub fn available_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "openai".into(),
            name: "OpenAI".into(),
            default_model: "whisper-1".into(),
        },
        ProviderInfo {
            id: "groq".into(),
            name: "Groq".into(),
            default_model: "whisper-large-v3".into(),
        },
        ProviderInfo {
            id: "deepgram".into(),
            name: "Deepgram".into(),
            default_model: "nova-2".into(),
        },
        ProviderInfo {
            id: "elevenlabs".into(),
            name: "ElevenLabs".into(),
            default_model: "scribe_v1".into(),
        },
        ProviderInfo {
            id: "gemini".into(),
            name: "Gemini".into(),
            default_model: "gemini-2.0-flash".into(),
        },
    ]
}

pub fn build_request_info(config: &CloudConfig, language: &str) -> RequestInfo {
    match config.provider {
        CloudProvider::OpenAI => {
            let base = config
                .base_url
                .as_deref()
                .unwrap_or("https://api.openai.com/v1");
            RequestInfo {
                url: format!("{}/audio/transcriptions", base),
                auth_header: format!("Bearer {}", config.api_key),
            }
        }
        CloudProvider::Groq => RequestInfo {
            url: "https://api.groq.com/openai/v1/audio/transcriptions".into(),
            auth_header: format!("Bearer {}", config.api_key),
        },
        CloudProvider::Deepgram => RequestInfo {
            url: format!(
                "https://api.deepgram.com/v1/listen?model={}&language={}",
                config.model, language
            ),
            auth_header: format!("Token {}", config.api_key),
        },
        CloudProvider::ElevenLabs => RequestInfo {
            url: "https://api.elevenlabs.io/v1/speech-to-text".into(),
            auth_header: format!("Bearer {}", config.api_key),
        },
        CloudProvider::Gemini => RequestInfo {
            url: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                config.model, config.api_key
            ),
            auth_header: String::new(),
        },
    }
}

pub fn parse_response(provider: CloudProvider, body: &str) -> Result<String, AppError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AppError::Network(e.to_string()))?;

    match provider {
        CloudProvider::OpenAI | CloudProvider::Groq | CloudProvider::ElevenLabs => v["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Network("Missing 'text' in response".into())),
        CloudProvider::Deepgram => v["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Network("Missing transcript in Deepgram response".into())),
        CloudProvider::Gemini => v["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Network("Missing text in Gemini response".into())),
    }
}

/// Transcribe audio file using cloud provider.
pub async fn transcribe(
    config: &CloudConfig,
    audio_data: &[u8],
    language: &str,
) -> Result<String, AppError> {
    let req_info = build_request_info(config, language);
    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .text("model", config.model.clone())
        .text("language", language.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(audio_data.to_vec())
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .unwrap(),
        );

    let mut request = client.post(&req_info.url);
    if !req_info.auth_header.is_empty() {
        request = request.header("Authorization", &req_info.auth_header);
    }

    let response = request.multipart(form).send().await?;
    let body = response.text().await?;

    parse_response(config.provider.clone(), &body)
}
