use yumo_core::denoiser;
use yumo_core::denoiser::Denoiser;

/// Helper: resolve the DTLN model directory from the project resources.
fn model_dir() -> Option<String> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dir = format!("{}/resources", manifest);
    let p1 = format!("{}/dtln_1.onnx", dir);
    let p2 = format!("{}/dtln_2.onnx", dir);
    if std::path::Path::new(&p1).exists() && std::path::Path::new(&p2).exists() {
        Some(dir)
    } else {
        None
    }
}

#[test]
fn test_passthrough_denoiser() {
    let d = denoiser::PassthroughDenoiser;
    let input = vec![0.1f32, 0.2, 0.3, -0.1, -0.2];
    let output = d.process(&input, 16000).unwrap();
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input);
}

#[test]
fn test_process_empty_audio() {
    let d = denoiser::PassthroughDenoiser;
    let output = d.process(&[], 16000).unwrap();
    assert!(output.is_empty());

    // Also test DtlnDenoiser with empty input if models available
    if let Some(dir) = model_dir() {
        let dtln = denoiser::DtlnDenoiser::new(&dir).unwrap();
        let output = dtln.process(&[], 16000).unwrap();
        assert!(output.is_empty());
    }
}

#[test]
fn test_denoiser_disabled_passthrough() {
    let config = denoiser::DenoiserConfig {
        enabled: false,
        model_dir: Some("/tmp/nonexistent".into()),
    };
    let input = vec![0.5f32; 1600];
    let output = denoiser::process_or_passthrough(&config, &input, 16000).unwrap();
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input);
}

#[test]
fn test_denoiser_enabled_no_model_fallback() {
    let config = denoiser::DenoiserConfig {
        enabled: true,
        model_dir: Some("/tmp/no_such_models_here_42".into()),
    };
    let input = vec![0.3f32; 800];
    let output = denoiser::process_or_passthrough(&config, &input, 16000).unwrap();
    // Should fall back to passthrough without error
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input);
}

#[test]
fn test_denoiser_enabled_no_model_dir() {
    let config = denoiser::DenoiserConfig {
        enabled: true,
        model_dir: None,
    };
    let input = vec![0.3f32; 800];
    let output = denoiser::process_or_passthrough(&config, &input, 16000).unwrap();
    assert_eq!(output.len(), input.len());
    assert_eq!(output, input);
}

#[test]
fn test_dtln_denoiser_loads_and_processes() {
    let dir = match model_dir() {
        Some(d) => d,
        None => {
            println!("DTLN models not found, skipping test_dtln_denoiser_loads_and_processes");
            return;
        }
    };

    let dtln = denoiser::DtlnDenoiser::new(&dir).unwrap();

    // Process silence (512 zeros = exactly 1 frame)
    let silence = vec![0.0f32; 512];
    let output = dtln.process(&silence, 16000).unwrap();
    assert_eq!(output.len(), silence.len());

    // Output of silence should be very quiet
    let max_abs = output.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    println!("silence max_abs output: {}", max_abs);
    assert!(max_abs < 1.0, "output of silence should be bounded");
}

#[test]
fn test_dtln_denoiser_preserves_length() {
    let dir = match model_dir() {
        Some(d) => d,
        None => {
            println!("DTLN models not found, skipping test_dtln_denoiser_preserves_length");
            return;
        }
    };

    let dtln = denoiser::DtlnDenoiser::new(&dir).unwrap();

    let test_lengths = [512, 640, 1000, 1024, 2048, 8000, 16000, 16001, 513, 700];
    for &len in &test_lengths {
        // Use a simple sine wave as test signal
        let input: Vec<f32> = (0..len)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.5)
            .collect();
        let output = dtln.process(&input, 16000).unwrap();
        assert_eq!(
            output.len(),
            input.len(),
            "length mismatch for input size {}",
            len
        );
    }
}

#[test]
fn test_dtln_denoiser_short_audio() {
    let dir = match model_dir() {
        Some(d) => d,
        None => {
            println!("DTLN models not found, skipping test_dtln_denoiser_short_audio");
            return;
        }
    };

    let dtln = denoiser::DtlnDenoiser::new(&dir).unwrap();

    // Audio shorter than 1 frame (512 samples)
    let short_inputs = [1, 10, 100, 256, 511];
    for &len in &short_inputs {
        let input = vec![0.1f32; len];
        let output = dtln.process(&input, 16000).unwrap();
        assert_eq!(
            output.len(),
            input.len(),
            "length mismatch for short audio size {}",
            len
        );
    }
}
