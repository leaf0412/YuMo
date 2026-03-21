pub mod traits;
pub mod types;

#[cfg(target_os = "macos")]
pub mod macos;

pub use types::*;
pub use traits::*;

#[cfg(target_os = "macos")]
pub use macos::recorder;
#[cfg(target_os = "macos")]
pub use macos::audio_ctrl;
#[cfg(target_os = "macos")]
pub use macos::paster;
#[cfg(target_os = "macos")]
pub use macos::permissions;
#[cfg(target_os = "macos")]
pub use macos::keychain;
