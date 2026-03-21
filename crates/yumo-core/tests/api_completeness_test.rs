//! Verifies all yumo-core public modules and key types are accessible.
//! This catches broken re-exports after crate extraction.

#![allow(unused_imports)]

// All modules should be importable
use yumo_core::cloud;
use yumo_core::daemon;
use yumo_core::daemon_client;
use yumo_core::db;
use yumo_core::denoiser;
use yumo_core::downloader;
use yumo_core::enhancer;
use yumo_core::error;
use yumo_core::mask;
use yumo_core::pipeline;
use yumo_core::state;
use yumo_core::text_processor;
use yumo_core::transcriber;
use yumo_core::vad;

// macOS-only modules
#[cfg(target_os = "macos")]
use yumo_core::audio_ctrl;
#[cfg(target_os = "macos")]
use yumo_core::keychain;
#[cfg(target_os = "macos")]
use yumo_core::paster;
#[cfg(target_os = "macos")]
use yumo_core::permissions;
#[cfg(target_os = "macos")]
use yumo_core::recorder;

#[test]
fn test_all_modules_accessible() {
    // error
    let _ = error::AppError::InvalidInput("test".into());

    // mask
    let _ = mask::mask("test");

    // pipeline
    let _ = pipeline::PipelineState::Idle;
    let _ = pipeline::Action::StartRecording;

    // text_processor
    let _ = text_processor::capitalize_sentences("test");

    // vad
    let _ = std::mem::size_of::<vad::VadResult>();

    // transcriber
    let _ = std::mem::size_of::<transcriber::ModelInfo>();
    let _ = std::mem::size_of::<transcriber::ModelProvider>();

    // cloud
    let _ = std::mem::size_of::<cloud::CloudProvider>();
    let _ = std::mem::size_of::<cloud::CloudConfig>();

    // enhancer
    let _ = std::mem::size_of::<enhancer::LLMProvider>();
    let _ = std::mem::size_of::<enhancer::EnhancerConfig>();

    // db
    let _ = std::mem::size_of::<db::TranscriptionRecord>();
    let _ = std::mem::size_of::<db::Statistics>();

    // denoiser
    let _ = std::mem::size_of::<denoiser::PassthroughDenoiser>();
    let _ = std::mem::size_of::<denoiser::DenoiserConfig>();

    // state
    let _ = state::AppPaths::defaults();

    // daemon
    let _ = std::mem::size_of::<daemon::DaemonManager>();
    let _ = std::mem::size_of::<daemon::DaemonResponse>();
    let _ = std::mem::size_of::<daemon::DaemonEventCallback>();

    // daemon_client
    let _ = std::mem::size_of::<daemon_client::DaemonResponse>();
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_modules_accessible() {
    // recorder types
    let _ = std::mem::size_of::<recorder::AudioInputDevice>();
    let _ = std::mem::size_of::<recorder::AudioData>();
    let _ = std::mem::size_of::<recorder::AudioLevel>();

    // These modules are function-based; just verify they compiled
    let _ = std::any::type_name::<fn()>();
    // audio_ctrl, paster, permissions, keychain modules are accessible
    // (verified by the `use` imports above compiling successfully)
}

#[test]
fn test_cloud_provider_variants() {
    // Verify all CloudProvider variants exist
    let providers = vec![
        cloud::CloudProvider::OpenAI,
        cloud::CloudProvider::Groq,
        cloud::CloudProvider::Deepgram,
        cloud::CloudProvider::ElevenLabs,
        cloud::CloudProvider::Gemini,
    ];
    assert_eq!(providers.len(), 5);
}

#[test]
fn test_llm_provider_variants() {
    let providers = vec![
        enhancer::LLMProvider::OpenAI,
        enhancer::LLMProvider::Anthropic,
        enhancer::LLMProvider::Ollama,
    ];
    assert_eq!(providers.len(), 3);
}

#[test]
fn test_model_provider_variants() {
    let local = transcriber::ModelProvider::Local;
    assert!(local.is_local());
    assert!(!local.is_cloud());

    let cloud = transcriber::ModelProvider::Groq;
    assert!(cloud.is_cloud());
    assert!(!cloud.is_local());

    let mlx = transcriber::ModelProvider::MlxWhisper;
    assert!(mlx.is_local());
    assert!(mlx.needs_daemon());
}
