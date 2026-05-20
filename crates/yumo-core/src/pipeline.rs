use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    Recording,
    Processing,
    Transcribing,
    Pasting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    StartRecording,
    StopRecording,
    Cancel,
    ProcessingComplete,
    TranscriptionComplete,
    PasteComplete,
}

pub fn transition(state: PipelineState, action: Action) -> PipelineState {
    match (state, action) {
        (PipelineState::Idle, Action::StartRecording) => PipelineState::Recording,
        (PipelineState::Recording, Action::StopRecording) => PipelineState::Processing,
        (PipelineState::Processing, Action::ProcessingComplete) => PipelineState::Transcribing,
        (PipelineState::Recording, Action::Cancel) => PipelineState::Idle,
        (PipelineState::Transcribing, Action::TranscriptionComplete) => PipelineState::Pasting,
        (PipelineState::Pasting, Action::PasteComplete) => PipelineState::Idle,
        // Cancel from any state goes to Idle
        (_, Action::Cancel) => PipelineState::Idle,
        // Invalid transitions: stay in current state
        _ => state,
    }
}
