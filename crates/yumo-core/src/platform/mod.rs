pub mod traits;
pub mod types;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

pub use types::*;
pub use traits::*;

#[cfg(target_os = "macos")]
pub use macos::recorder;
#[cfg(target_os = "macos")]
pub use macos::recorder::RecordingHandle;
#[cfg(target_os = "macos")]
pub use macos::audio_ctrl;
#[cfg(target_os = "macos")]
pub use macos::paster;
#[cfg(target_os = "macos")]
pub use macos::permissions;
#[cfg(target_os = "macos")]
pub use macos::keychain;

#[cfg(target_os = "windows")]
pub use windows::recorder;
#[cfg(target_os = "windows")]
pub use windows::recorder::RecordingHandle;
#[cfg(target_os = "windows")]
pub use windows::audio_ctrl;
#[cfg(target_os = "windows")]
pub use windows::paster;
#[cfg(target_os = "windows")]
pub use windows::permissions;
#[cfg(target_os = "windows")]
pub use windows::keychain;

#[cfg(target_os = "linux")]
pub use linux::recorder;
#[cfg(target_os = "linux")]
pub use linux::recorder::RecordingHandle;
#[cfg(target_os = "linux")]
pub use linux::audio_ctrl;
#[cfg(target_os = "linux")]
pub use linux::paster;
#[cfg(target_os = "linux")]
pub use linux::permissions;
#[cfg(target_os = "linux")]
pub use linux::keychain;
