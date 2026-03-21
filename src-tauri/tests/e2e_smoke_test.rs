use yumo_lib::{db, text_processor, enhancer, pipeline, transcriber, recorder, audio_ctrl};
use tempfile::TempDir;

#[test]
fn test_full_app_lifecycle() {
    // 1. Init DB
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    // 2. List predefined models
    let models = transcriber::predefined_models();
    assert!(!models.is_empty());

    // 3. List audio devices
    let devices = recorder::list_input_devices();
    assert!(!devices.is_empty());

    // 4. Add vocabulary
    let vocab_id = db::add_vocabulary(&conn, "Kubernetes").unwrap();
    assert!(!vocab_id.is_empty());

    // 5. Add replacement
    let rep_id = db::set_replacement(&conn, "k8s", "Kubernetes").unwrap();
    assert!(!rep_id.is_empty());

    // 6. Settings roundtrip
    db::update_setting(&conn, "language", &serde_json::json!("en")).unwrap();
    let lang = db::get_setting(&conn, "language").unwrap();
    assert_eq!(lang, Some(serde_json::json!("en")));

    // 7. Prompt roundtrip
    let prompt_id = db::add_prompt(&conn, "Test", "sys", "user: {{text}}", false).unwrap();
    let prompts = db::list_prompts(&conn).unwrap();
    assert!(prompts.iter().any(|p| p.id == prompt_id));

    // 8. Text processing
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::process_text("i use k8s", &replacements, true);
    assert_eq!(result, "I use Kubernetes");

    // 9. Pipeline state machine
    let state = pipeline::transition(pipeline::PipelineState::Idle, pipeline::Action::StartRecording);
    assert_eq!(state, pipeline::PipelineState::Recording);

    // 10. Audio control
    let _muted = audio_ctrl::is_system_muted();

    // 11. Enhancer prompt building
    let vocab = vec!["Kubernetes".to_string()];
    let (sys, user) = enhancer::build_prompt("Fix text", "Fix: {{text}}", "hello k8s", &vocab);
    assert!(user.contains("hello k8s"));
    assert!(user.contains("Kubernetes"));

    // 12. Insert transcription to DB
    let t_id = db::insert_transcription(&conn, "test transcription", Some("enhanced"), 2.5, "ggml-base", 2, None).unwrap();
    let result = db::get_transcriptions(&conn, None, None, 20).unwrap();
    assert_eq!(result.items.len(), 1);

    // 13. Delete transcription
    db::delete_transcription(&conn, &t_id).unwrap();
    let result = db::get_transcriptions(&conn, None, None, 20).unwrap();
    assert_eq!(result.items.len(), 0);

    // 14. Cleanup
    db::delete_vocabulary(&conn, &vocab_id).unwrap();
    db::delete_replacement(&conn, &rep_id).unwrap();
    db::delete_prompt(&conn, &prompt_id).unwrap();
}
