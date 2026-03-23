use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInputDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
}

#[derive(Debug, Clone)]
pub struct AudioData {
    pub pcm_samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Opaque handle for a pre-initialized recording session.
/// The platform stores its own prepared state inside `inner`.
pub struct PreparedRecordingHandle {
    pub(crate) inner: Box<dyn std::any::Any + Send>,
    pub device_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub microphone: bool,
    pub accessibility: bool,
    /// Linux only: availability of paste tools (xdotool, wtype).
    /// None on macOS/Windows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paste_tools: Option<PasteToolsStatus>,
}

/// Which paste simulation tools are available on Linux.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteToolsStatus {
    pub xdotool: bool,
    pub wtype: bool,
}
