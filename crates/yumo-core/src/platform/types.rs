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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub microphone: bool,
    pub accessibility: bool,
}
