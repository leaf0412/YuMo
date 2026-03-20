use yumo_lib::text_processor;

#[test]
fn test_apply_replacements() {
    let replacements = vec![
        ("k8s".to_string(), "Kubernetes".to_string()),
        ("js".to_string(), "JavaScript".to_string()),
    ];
    let result = text_processor::apply_replacements("I use k8s and js daily", &replacements);
    assert_eq!(result, "I use Kubernetes and JavaScript daily");
}

#[test]
fn test_replacement_case_insensitive() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::apply_replacements("K8S is great", &replacements);
    assert_eq!(result, "Kubernetes is great");
}

#[test]
fn test_replacement_word_boundary() {
    let replacements = vec![("js".to_string(), "JavaScript".to_string())];
    // Should NOT replace "json" → "JavaScripton"
    let result = text_processor::apply_replacements("I work with json files", &replacements);
    assert_eq!(result, "I work with json files");
}

#[test]
fn test_replacement_at_string_boundaries() {
    let replacements = vec![("js".to_string(), "JavaScript".to_string())];
    let result = text_processor::apply_replacements("js is popular", &replacements);
    assert_eq!(result, "JavaScript is popular");

    let result = text_processor::apply_replacements("I love js", &replacements);
    assert_eq!(result, "I love JavaScript");
}

#[test]
fn test_auto_capitalize() {
    let result = text_processor::capitalize_sentences("hello world. this is a test. yes.");
    assert_eq!(result, "Hello world. This is a test. Yes.");
}

#[test]
fn test_capitalize_after_question_mark() {
    let result = text_processor::capitalize_sentences("what? really. ok.");
    assert_eq!(result, "What? Really. Ok.");
}

#[test]
fn test_capitalize_after_exclamation() {
    let result = text_processor::capitalize_sentences("wow! that is great.");
    assert_eq!(result, "Wow! That is great.");
}

#[test]
fn test_capitalize_empty_string() {
    let result = text_processor::capitalize_sentences("");
    assert_eq!(result, "");
}

#[test]
fn test_capitalize_already_capitalized() {
    let result = text_processor::capitalize_sentences("Hello World.");
    assert_eq!(result, "Hello World.");
}

#[test]
fn test_process_text_combined() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        true, // auto_capitalize
    );
    assert_eq!(result, "I deploy to Kubernetes. It works.");
}

#[test]
fn test_process_text_no_capitalize() {
    let replacements = vec![("k8s".to_string(), "Kubernetes".to_string())];
    let result = text_processor::process_text(
        "i deploy to k8s. it works.",
        &replacements,
        false,
    );
    assert_eq!(result, "i deploy to Kubernetes. it works.");
}

#[test]
fn test_no_replacements() {
    let result = text_processor::apply_replacements("hello world", &[]);
    assert_eq!(result, "hello world");
}
