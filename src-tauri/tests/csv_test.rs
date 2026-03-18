use voiceink_tauri_lib::db;
use tempfile::TempDir;

#[test]
fn test_export_import_vocabulary_csv_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    db::add_vocabulary(&conn, "Kubernetes").unwrap();
    db::add_vocabulary(&conn, "TypeScript").unwrap();

    // Export
    let csv_path = tmp.path().join("vocab.csv");
    db::export_vocabulary_csv(&conn, &csv_path).unwrap();
    assert!(csv_path.exists());

    // Clear and reimport
    let conn2 = db::init_database(&tmp.path().join("test2.db")).unwrap();
    db::import_vocabulary_csv(&conn2, &csv_path).unwrap();
    let words = db::get_vocabulary(&conn2).unwrap();
    assert_eq!(words.len(), 2);
    assert!(words.iter().any(|w| w.word == "Kubernetes"));
    assert!(words.iter().any(|w| w.word == "TypeScript"));
}

#[test]
fn test_export_import_replacements_csv_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let conn = db::init_database(&tmp.path().join("test.db")).unwrap();

    db::set_replacement(&conn, "k8s", "Kubernetes").unwrap();
    db::set_replacement(&conn, "js", "JavaScript").unwrap();

    let csv_path = tmp.path().join("replacements.csv");
    db::export_replacements_csv(&conn, &csv_path).unwrap();
    assert!(csv_path.exists());

    let conn2 = db::init_database(&tmp.path().join("test2.db")).unwrap();
    db::import_replacements_csv(&conn2, &csv_path).unwrap();
    let reps = db::get_replacements(&conn2).unwrap();
    assert_eq!(reps.len(), 2);
    assert!(reps.iter().any(|r| r.original == "k8s" && r.replacement == "Kubernetes"));
}
