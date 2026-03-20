use yumo_lib::denoiser;
use yumo_lib::denoiser::Denoiser;

#[test]
fn test_passthrough_denoiser() {
    let d = denoiser::PassthroughDenoiser;
    let input = vec![0.1f32, 0.2, 0.3, -0.1, -0.2];
    let output = d.process(&input, 16000).unwrap();
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input);
}

#[test]
fn test_denoiser_config() {
    let config = denoiser::DenoiserConfig {
        enabled: true,
        model_path: Some("/tmp/fake_model.onnx".into()),
    };
    assert!(config.enabled);
    assert!(config.model_path.is_some());
}

#[test]
fn test_denoiser_disabled_config() {
    let config = denoiser::DenoiserConfig {
        enabled: false,
        model_path: None,
    };
    assert!(!config.enabled);
}

#[test]
fn test_process_or_passthrough_when_disabled() {
    let config = denoiser::DenoiserConfig {
        enabled: false,
        model_path: None,
    };
    let input = vec![0.5f32; 16000];
    let output = denoiser::process_or_passthrough(&config, &input, 16000).unwrap();
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input); // passthrough when disabled
}

#[test]
fn test_process_empty_audio() {
    let d = denoiser::PassthroughDenoiser;
    let output = d.process(&[], 16000).unwrap();
    assert!(output.is_empty());
}
