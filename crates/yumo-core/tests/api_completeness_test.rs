//! Verifies all yumo-core public modules and key types are accessible.
//! This catches broken re-exports after crate extraction.

#![allow(unused_imports)]

// All modules should be importable
use yumo_core::daemon;
use yumo_core::daemon_client;
use yumo_core::db;
use yumo_core::downloader;
use yumo_core::error;
use yumo_core::mask;
use yumo_core::pipeline;
use yumo_core::state;
use yumo_core::text_processor;
use yumo_core::transcriber;
use yumo_core::vad;
use yumo_core::audio_io;

// macOS-only modules
#[cfg(target_os = "macos")]
use yumo_core::platform::audio_ctrl;
#[cfg(target_os = "macos")]
use yumo_core::platform::paster;
#[cfg(target_os = "macos")]
use yumo_core::platform::permissions;
#[cfg(target_os = "macos")]
use yumo_core::platform::recorder;

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

    // db
    let _ = std::mem::size_of::<db::TranscriptionRecord>();
    let _ = std::mem::size_of::<db::Statistics>();

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
    // recorder types (from platform::types, re-exported via platform::*)
    let _ = std::mem::size_of::<yumo_core::platform::AudioInputDevice>();
    let _ = std::mem::size_of::<yumo_core::platform::AudioData>();
    let _ = std::mem::size_of::<yumo_core::platform::AudioLevel>();

    // audio_ctrl, paster, permissions modules accessible (verified by `use` above)
}

#[test]
fn test_model_provider_variants() {
    let local = transcriber::ModelProvider::Local;
    assert!(local.is_local());

    let mlx = transcriber::ModelProvider::MlxWhisper;
    assert!(mlx.is_local());
    assert!(mlx.needs_daemon());
}
