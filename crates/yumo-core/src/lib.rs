// yumo-core: 语墨核心业务逻辑库

pub mod platform;

pub mod daemon;
pub mod daemon_client;
pub mod error;
pub mod mask;
pub mod pipeline;
pub mod text_processor;
pub mod transcriber;
pub mod cloud;
pub mod enhancer;
pub mod downloader;
pub mod vad;
pub mod db;
pub mod denoiser;
pub mod state;

#[cfg(target_os = "macos")]
pub mod recorder;
#[cfg(target_os = "macos")]
pub mod audio_ctrl;
#[cfg(target_os = "macos")]
pub mod paster;
#[cfg(target_os = "macos")]
pub mod permissions;
#[cfg(target_os = "macos")]
pub mod keychain;
