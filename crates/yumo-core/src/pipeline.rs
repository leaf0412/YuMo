use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    Recording,
    Processing,
    Transcribing,
    Enhancing,
    Pasting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    StartRecording,
    StopRecording,
    Cancel,
    ProcessingComplete,
    TranscriptionComplete,
    EnhancementComplete,
    PasteComplete,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub enhancement_enabled: bool,
}

/// Default transition without config (enhancement disabled).
pub fn transition(state: PipelineState, action: Action) -> PipelineState {
    let config = PipelineConfig { enhancement_enabled: false };
    transition_with_config(state, action, &config)
}

/// Transition with config (determines if enhancement step is included).
pub fn transition_with_config(state: PipelineState, action: Action, config: &PipelineConfig) -> PipelineState {
    match (state, action) {
        (PipelineState::Idle, Action::StartRecording) => PipelineState::Recording,
        (PipelineState::Recording, Action::StopRecording) => PipelineState::Processing,
        (PipelineState::Processing, Action::ProcessingComplete) => PipelineState::Transcribing,
        (PipelineState::Recording, Action::Cancel) => PipelineState::Idle,
        (PipelineState::Transcribing, Action::TranscriptionComplete) => {
            if config.enhancement_enabled {
                PipelineState::Enhancing
            } else {
                PipelineState::Pasting
            }
        }
        (PipelineState::Enhancing, Action::EnhancementComplete) => PipelineState::Pasting,
        (PipelineState::Pasting, Action::PasteComplete) => PipelineState::Idle,
        // Cancel from any state goes to Idle
        (_, Action::Cancel) => PipelineState::Idle,
        // Invalid transitions: stay in current state
        _ => state,
    }
}
