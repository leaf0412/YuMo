use voiceink_tauri_lib::pipeline;

#[test]
fn test_pipeline_state_transitions() {
    let mut state = pipeline::PipelineState::Idle;

    state = pipeline::transition(state, pipeline::Action::StartRecording);
    assert_eq!(state, pipeline::PipelineState::Recording);

    state = pipeline::transition(state, pipeline::Action::StopRecording);
    assert_eq!(state, pipeline::PipelineState::Transcribing);

    state = pipeline::transition(state, pipeline::Action::TranscriptionComplete);
    assert_eq!(state, pipeline::PipelineState::Pasting);

    state = pipeline::transition(state, pipeline::Action::PasteComplete);
    assert_eq!(state, pipeline::PipelineState::Idle);
}

#[test]
fn test_pipeline_cancel_from_recording() {
    let mut state = pipeline::PipelineState::Recording;
    state = pipeline::transition(state, pipeline::Action::Cancel);
    assert_eq!(state, pipeline::PipelineState::Idle);
}

#[test]
fn test_pipeline_with_enhancement_enabled() {
    let config = pipeline::PipelineConfig { enhancement_enabled: true };
    let mut state = pipeline::PipelineState::Transcribing;

    state = pipeline::transition_with_config(state, pipeline::Action::TranscriptionComplete, &config);
    assert_eq!(state, pipeline::PipelineState::Enhancing);

    state = pipeline::transition(state, pipeline::Action::EnhancementComplete);
    assert_eq!(state, pipeline::PipelineState::Pasting);

    state = pipeline::transition(state, pipeline::Action::PasteComplete);
    assert_eq!(state, pipeline::PipelineState::Idle);
}

#[test]
fn test_pipeline_without_enhancement() {
    let config = pipeline::PipelineConfig { enhancement_enabled: false };
    let mut state = pipeline::PipelineState::Transcribing;

    state = pipeline::transition_with_config(state, pipeline::Action::TranscriptionComplete, &config);
    assert_eq!(state, pipeline::PipelineState::Pasting);
}

#[test]
fn test_invalid_transition_stays_in_state() {
    // Can't stop recording from idle
    let state = pipeline::transition(pipeline::PipelineState::Idle, pipeline::Action::StopRecording);
    assert_eq!(state, pipeline::PipelineState::Idle);

    // Can't start recording while already recording
    let state = pipeline::transition(pipeline::PipelineState::Recording, pipeline::Action::StartRecording);
    assert_eq!(state, pipeline::PipelineState::Recording);
}
