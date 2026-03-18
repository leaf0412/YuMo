use tempfile::TempDir;

// The lib is named voiceink_tauri_lib in Cargo.toml
use voiceink_tauri_lib::db;

#[test]
fn test_init_creates_all_tables() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let conn = db::init_database(&db_path).unwrap();

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(tables.contains(&"transcriptions".to_string()));
    assert!(tables.contains(&"vocabulary".to_string()));
    assert!(tables.contains(&"replacements".to_string()));
    assert!(tables.contains(&"prompts".to_string()));
    assert!(tables.contains(&"settings".to_string()));
}

#[test]
fn test_insert_and_query_transcription() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    let id = db::insert_transcription(&conn, "hello world", None, 2.5, "ggml-base", 2).unwrap();
    let result = db::get_transcriptions(&conn, None, None, 20).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].text, "hello world");
    assert_eq!(result.items[0].id, id);
}

#[test]
fn test_fulltext_search_transcriptions() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    db::insert_transcription(&conn, "the quick brown fox", None, 1.0, "base", 4).unwrap();
    db::insert_transcription(&conn, "hello world goodbye", None, 1.0, "base", 3).unwrap();

    let result = db::get_transcriptions(&conn, None, Some("fox"), 20).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].text, "the quick brown fox");
}

#[test]
fn test_cursor_pagination() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    for i in 0..5 {
        db::insert_transcription(&conn, &format!("entry {}", i), None, 1.0, "base", 2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let page1 = db::get_transcriptions(&conn, None, None, 3).unwrap();
    assert_eq!(page1.items.len(), 3);
    assert!(page1.next_cursor.is_some());

    let page2 = db::get_transcriptions(&conn, page1.next_cursor.as_deref(), None, 3).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert!(page2.next_cursor.is_none());
}

#[test]
fn test_delete_transcription() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    let id = db::insert_transcription(&conn, "delete me", None, 1.0, "base", 2).unwrap();
    db::delete_transcription(&conn, &id).unwrap();
    let result = db::get_transcriptions(&conn, None, None, 20).unwrap();
    assert_eq!(result.items.len(), 0);
}

#[test]
fn test_vocabulary_crud() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    let id = db::add_vocabulary(&conn, "Kubernetes").unwrap();
    let words = db::get_vocabulary(&conn).unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].word, "Kubernetes");

    db::delete_vocabulary(&conn, &id).unwrap();
    assert_eq!(db::get_vocabulary(&conn).unwrap().len(), 0);
}

#[test]
fn test_replacements_crud() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    let id = db::set_replacement(&conn, "k8s", "Kubernetes").unwrap();
    let reps = db::get_replacements(&conn).unwrap();
    assert_eq!(reps.len(), 1);
    assert_eq!(reps[0].original, "k8s");
    assert_eq!(reps[0].replacement, "Kubernetes");

    db::delete_replacement(&conn, &id).unwrap();
    assert_eq!(db::get_replacements(&conn).unwrap().len(), 0);
}

#[test]
fn test_settings_crud() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    db::update_setting(&conn, "language", &serde_json::json!("zh")).unwrap();
    let val = db::get_setting(&conn, "language").unwrap();
    assert_eq!(val, Some(serde_json::json!("zh")));

    assert_eq!(db::get_setting(&conn, "nonexistent").unwrap(), None);
}

#[test]
fn test_prompts_crud() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    let id = db::add_prompt(&conn, "Fix Grammar", "You fix grammar.", "Fix: {{text}}", false).unwrap();
    let prompts = db::list_prompts(&conn).unwrap();
    assert!(prompts.iter().any(|p| p.name == "Fix Grammar"));

    db::update_prompt(&conn, &id, "Fix Grammar v2", "Updated.", "Fix v2: {{text}}").unwrap();
    let prompts = db::list_prompts(&conn).unwrap();
    assert!(prompts.iter().any(|p| p.name == "Fix Grammar v2"));

    db::delete_prompt(&conn, &id).unwrap();
    assert!(!db::list_prompts(&conn).unwrap().iter().any(|p| p.id == id));
}

#[test]
fn test_auto_cleanup_by_days() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    // Insert old record
    conn.execute(
        "INSERT INTO transcriptions (id, text, timestamp, duration, model_name, word_count)
         VALUES (?1, ?2, datetime('now', '-10 days'), ?3, ?4, ?5)",
        rusqlite::params!["old-id", "old text", 1.0, "base", 2],
    ).unwrap();

    db::insert_transcription(&conn, "new text", None, 1.0, "base", 2).unwrap();

    let deleted = db::cleanup_old_transcriptions(&conn, 7).unwrap();
    assert_eq!(deleted, 1);

    let result = db::get_transcriptions(&conn, None, None, 20).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].text, "new text");
}
