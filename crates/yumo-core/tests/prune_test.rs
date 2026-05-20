//! prune_older_than_days 集成测试 — 真临时 SQLite + 真临时 WAV 文件,
//! 不 mock 任何东西。

use rusqlite::params;
use tempfile::TempDir;
use yumo_core::db;

fn open_db(dir: &TempDir) -> rusqlite::Connection {
    db::init_database(&dir.path().join("test.db")).unwrap()
}

/// 插入一条 transcription, 指定 timestamp (UTC), 返回 id。
fn insert_at(conn: &rusqlite::Connection, ts: &str, recording: Option<&str>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO transcriptions (id, text, enhanced_text, timestamp, duration, model_name, word_count, recording_path)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
        params![id, "hello", ts, 1.0_f64, "test-model", 1_i32, recording],
    ).unwrap();
    id
}

#[test]
fn prune_no_rows_when_all_recent() {
    let dir = TempDir::new().unwrap();
    let conn = open_db(&dir);

    let now = chrono::Utc::now();
    let recent = (now - chrono::Duration::days(3))
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    insert_at(&conn, &recent, None);
    insert_at(&conn, &recent, None);

    let summary = db::prune_older_than_days(&conn, 30).unwrap();
    assert_eq!(summary.rows_deleted, 0);
    assert_eq!(summary.files_deleted, 0);

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM transcriptions", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn prune_deletes_only_older_than_cutoff() {
    let dir = TempDir::new().unwrap();
    let conn = open_db(&dir);

    let now = chrono::Utc::now();
    let old = (now - chrono::Duration::days(45))
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    let recent = (now - chrono::Duration::days(5))
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    insert_at(&conn, &old, None);
    insert_at(&conn, &old, None);
    let keep = insert_at(&conn, &recent, None);

    let summary = db::prune_older_than_days(&conn, 30).unwrap();
    assert_eq!(summary.rows_deleted, 2);

    let remaining: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM transcriptions").unwrap();
        let iter = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        iter.map(|r| r.unwrap()).collect()
    };
    assert_eq!(remaining, vec![keep]);
}

#[test]
fn prune_removes_associated_wav_and_txt_files() {
    let dir = TempDir::new().unwrap();
    let conn = open_db(&dir);

    // 模拟 pipeline 落盘: WAV + 旁边的 .txt
    let wav_path = dir.path().join("recording_old.wav");
    let txt_path = dir.path().join("recording_old.txt");
    std::fs::write(&wav_path, b"fake wav").unwrap();
    std::fs::write(&txt_path, b"fake transcript").unwrap();

    let old = (chrono::Utc::now() - chrono::Duration::days(60))
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    insert_at(&conn, &old, Some(wav_path.to_str().unwrap()));

    let summary = db::prune_older_than_days(&conn, 30).unwrap();
    assert_eq!(summary.rows_deleted, 1);
    assert_eq!(summary.files_deleted, 1);
    assert_eq!(summary.files_failed, 0);
    assert!(!wav_path.exists(), "wav should be deleted");
    assert!(!txt_path.exists(), "txt sidecar should be deleted");
}

#[test]
fn prune_tolerates_missing_wav_file() {
    let dir = TempDir::new().unwrap();
    let conn = open_db(&dir);

    // recording_path 指向不存在的文件 — 不应当算失败 (孤儿记录是常见场景)
    let phantom = dir.path().join("recording_gone.wav");
    let old = (chrono::Utc::now() - chrono::Duration::days(60))
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    insert_at(&conn, &old, Some(phantom.to_str().unwrap()));

    let summary = db::prune_older_than_days(&conn, 30).unwrap();
    assert_eq!(summary.rows_deleted, 1);
    assert_eq!(summary.files_deleted, 0); // 文件本来就不在
    assert_eq!(summary.files_failed, 0); // ErrorKind::NotFound 不算失败
}

#[test]
fn prune_zero_days_deletes_everything_already_persisted() {
    let dir = TempDir::new().unwrap();
    let conn = open_db(&dir);

    let now_str = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    insert_at(&conn, &now_str, None);
    insert_at(&conn, &now_str, None);

    // days=0 → cutoff = 现在 (此函数被调用的瞬间)。已经插入的记录
    // 时间戳必然 < cutoff (即使只差几 μs), 全部命中删除。生产里不会
    // 用 0, 这里只验语义。
    let summary = db::prune_older_than_days(&conn, 0).unwrap();
    assert_eq!(summary.rows_deleted, 2);
}
