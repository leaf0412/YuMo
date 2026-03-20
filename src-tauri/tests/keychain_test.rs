use yumo_lib::keychain;

#[test]
fn test_store_and_retrieve_api_key() {
    let service = "com.voiceink.test";
    let account = "openai-test";

    keychain::store_key(service, account, "sk-test-key-123").unwrap();
    let key = keychain::get_key(service, account).unwrap();
    assert_eq!(key, Some("sk-test-key-123".to_string()));

    // Cleanup
    keychain::delete_key(service, account).unwrap();
    let key = keychain::get_key(service, account).unwrap();
    assert_eq!(key, None);
}

#[test]
fn test_get_nonexistent_key_returns_none() {
    let key = keychain::get_key("com.voiceink.test", "nonexistent-provider-xyz").unwrap();
    assert_eq!(key, None);
}

#[test]
fn test_update_existing_key() {
    let service = "com.voiceink.test";
    let account = "anthropic-test";

    keychain::store_key(service, account, "old-key").unwrap();
    keychain::store_key(service, account, "new-key").unwrap();
    let key = keychain::get_key(service, account).unwrap();
    assert_eq!(key, Some("new-key".to_string()));

    keychain::delete_key(service, account).unwrap();
}

#[test]
fn test_delete_nonexistent_key_is_ok() {
    // Should not error when deleting a key that doesn't exist
    let result = keychain::delete_key("com.voiceink.test", "never-existed-xyz");
    assert!(result.is_ok());
}
