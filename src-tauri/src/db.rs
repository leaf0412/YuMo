use std::collections::HashMap;
use std::path::Path;

use log::{error, info};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::mask;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRecord {
    pub id: String,
    pub text: String,
    pub enhanced_text: Option<String>,
    pub timestamp: String,
    pub duration: f64,
    pub model_name: String,
    pub word_count: i32,
    pub recording_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyWpm {
    pub date: String,
    pub wpm: f64,
    pub session_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpmStats {
    pub avg: f64,
    pub max: f64,
    pub min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub total_sessions: i64,
    pub total_words: i64,
    pub total_duration_seconds: f64,
    pub total_keystrokes_saved: i64,
    pub time_saved_minutes: f64,
    pub avg_wpm: f64,
    pub daily_wpm: Vec<DailyWpm>,
    pub wpm_stats: WpmStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult {
    pub items: Vec<TranscriptionRecord>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyWord {
    pub id: String,
    pub word: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub id: String,
    pub original: String,
    pub replacement: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub system_message: String,
    pub user_message_template: String,
    pub is_predefined: bool,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Database initialisation
// ---------------------------------------------------------------------------

pub fn init_database(path: &Path) -> Result<Connection, AppError> {
    info!("[db] init_database path={}", path.display());
    let conn = Connection::open(path).map_err(|e| {
        error!("[db] init_database open failed: {}", e);
        e
    })?;

    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS transcriptions (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            enhanced_text TEXT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            duration REAL NOT NULL,
            model_name TEXT NOT NULL,
            word_count INTEGER NOT NULL,
            recording_path TEXT
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS transcriptions_fts
            USING fts5(text, content=transcriptions, content_rowid=rowid);

        CREATE TRIGGER IF NOT EXISTS transcriptions_ai AFTER INSERT ON transcriptions BEGIN
            INSERT INTO transcriptions_fts(rowid, text) VALUES (new.rowid, new.text);
        END;

        CREATE TRIGGER IF NOT EXISTS transcriptions_ad AFTER DELETE ON transcriptions BEGIN
            INSERT INTO transcriptions_fts(transcriptions_fts, rowid, text)
                VALUES('delete', old.rowid, old.text);
        END;

        CREATE TABLE IF NOT EXISTS vocabulary (
            id TEXT PRIMARY KEY,
            word TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS replacements (
            id TEXT PRIMARY KEY,
            original TEXT NOT NULL,
            replacement TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS prompts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            system_message TEXT NOT NULL,
            user_message_template TEXT NOT NULL,
            is_predefined INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )?;

    // Migrations for existing databases
    let _ = conn.execute_batch(
        "ALTER TABLE transcriptions ADD COLUMN recording_path TEXT;",
    ); // Silently ignore if column already exists

    seed_predefined_prompts(&conn)?;

    info!("[db] init_database complete");
    Ok(conn)
}

fn seed_predefined_prompts(conn: &Connection) -> Result<(), AppError> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM prompts WHERE is_predefined = 1", [], |r| r.get(0))?;

    if count > 0 {
        return Ok(());
    }

    let seeds = [
        (
            "Clean Up",
            "You are a text editor. Clean up the transcribed text: fix typos, remove filler words, improve readability. Keep the original meaning.",
            "{{text}}",
        ),
        (
            "Formal",
            "Rewrite the following text in a formal, professional tone.",
            "{{text}}",
        ),
        (
            "Summarize",
            "Summarize the following text concisely.",
            "{{text}}",
        ),
    ];

    for (name, sys, usr) in &seeds {
        conn.execute(
            "INSERT INTO prompts (id, name, system_message, user_message_template, is_predefined)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![Uuid::new_v4().to_string(), name, sys, usr],
        )?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Transcriptions
// ---------------------------------------------------------------------------

pub fn insert_transcription(
    conn: &Connection,
    text: &str,
    enhanced_text: Option<&str>,
    duration: f64,
    model_name: &str,
    word_count: i32,
    recording_path: Option<&str>,
) -> Result<String, AppError> {
    info!("[db] insert_transcription model={} duration={:.1} word_count={} text={}", model_name, duration, word_count, mask::mask_text(text));
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    conn.execute(
        "INSERT INTO transcriptions (id, text, enhanced_text, timestamp, duration, model_name, word_count, recording_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, text, enhanced_text, now, duration, model_name, word_count, recording_path],
    )
    .map_err(|e| {
        error!("[db] insert_transcription failed: {}", e);
        e
    })?;
    Ok(id)
}

pub fn get_transcriptions(
    conn: &Connection,
    cursor: Option<&str>,
    query: Option<&str>,
    limit: usize,
) -> Result<PaginatedResult, AppError> {
    info!("[db] get_transcriptions limit={} cursor={:?} query={:?}", limit, cursor, query);
    let items: Vec<TranscriptionRecord> = match (cursor, query) {
        (None, None) => {
            let mut stmt = conn.prepare(
                "SELECT id, text, enhanced_text, timestamp, duration, model_name, word_count, recording_path
                 FROM transcriptions ORDER BY timestamp DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        (Some(cur), None) => {
            let mut stmt = conn.prepare(
                "SELECT id, text, enhanced_text, timestamp, duration, model_name, word_count, recording_path
                 FROM transcriptions WHERE timestamp < ?1 ORDER BY timestamp DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![cur, limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        (None, Some(q)) => {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.text, t.enhanced_text, t.timestamp, t.duration, t.model_name, t.word_count, t.recording_path
                 FROM transcriptions t
                 JOIN transcriptions_fts fts ON t.rowid = fts.rowid
                 WHERE transcriptions_fts MATCH ?1
                 ORDER BY t.timestamp DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![q, limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        (Some(cur), Some(q)) => {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.text, t.enhanced_text, t.timestamp, t.duration, t.model_name, t.word_count, t.recording_path
                 FROM transcriptions t
                 JOIN transcriptions_fts fts ON t.rowid = fts.rowid
                 WHERE transcriptions_fts MATCH ?1 AND t.timestamp < ?2
                 ORDER BY t.timestamp DESC LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![q, cur, limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
    };

    let next_cursor = if items.len() == limit {
        items.last().map(|r| r.timestamp.clone())
    } else {
        None
    };

    info!("[db] get_transcriptions returned {} items", items.len());
    Ok(PaginatedResult { items, next_cursor })
}

fn row_to_transcription(row: &rusqlite::Row) -> rusqlite::Result<TranscriptionRecord> {
    Ok(TranscriptionRecord {
        id: row.get(0)?,
        text: row.get(1)?,
        enhanced_text: row.get(2)?,
        timestamp: row.get(3)?,
        duration: row.get(4)?,
        model_name: row.get(5)?,
        word_count: row.get(6)?,
        recording_path: row.get(7)?,
    })
}

pub fn delete_transcription(conn: &Connection, id: &str) -> Result<(), AppError> {
    info!("[db] delete_transcription id={}", id);
    conn.execute("DELETE FROM transcriptions WHERE id = ?1", params![id])
        .map_err(|e| {
            error!("[db] delete_transcription failed: {}", e);
            e
        })?;
    Ok(())
}

pub fn delete_all_transcriptions(conn: &Connection) -> Result<(), AppError> {
    info!("[db] delete_all_transcriptions");
    conn.execute("DELETE FROM transcriptions", []).map_err(|e| {
        error!("[db] delete_all_transcriptions failed: {}", e);
        e
    })?;
    Ok(())
}

pub fn cleanup_old_transcriptions(conn: &Connection, days: i32) -> Result<usize, AppError> {
    info!("[db] cleanup_old_transcriptions days={}", days);
    let deleted = conn.execute(
        "DELETE FROM transcriptions WHERE timestamp < datetime('now', ?1)",
        params![format!("-{} days", days)],
    )
    .map_err(|e| {
        error!("[db] cleanup_old_transcriptions failed: {}", e);
        e
    })?;
    info!("[db] cleanup_old_transcriptions deleted={}", deleted);
    Ok(deleted)
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}'
    )
}

fn calc_keystrokes(text: &str) -> i64 {
    let mut cjk_count: i64 = 0;
    let mut non_cjk_word_count: i64 = 0;
    let mut in_word = false;
    for c in text.chars() {
        if is_cjk(c) {
            cjk_count += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            non_cjk_word_count += 1;
            in_word = true;
        }
    }
    cjk_count * 6 + non_cjk_word_count * 5
}

fn count_words(text: &str) -> i64 {
    let mut count: i64 = 0;
    let mut in_word = false;
    for c in text.chars() {
        if is_cjk(c) {
            count += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            count += 1;
            in_word = true;
        }
    }
    count
}

fn calc_typing_time_minutes(text: &str) -> f64 {
    let mut cjk_count = 0i64;
    let mut other_word_count = 0i64;
    let mut in_word = false;
    for c in text.chars() {
        if is_cjk(c) {
            cjk_count += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            other_word_count += 1;
            in_word = true;
        }
    }
    (cjk_count as f64 / 100.0) + (other_word_count as f64 / 40.0)
}

pub fn get_statistics(conn: &Connection, days: Option<i64>) -> Result<Statistics, AppError> {
    info!("[db] get_statistics days={:?}", days);

    let date_filter = days.map(|d| format!("-{} days", d));

    let (total_sessions, total_duration): (i64, f64) = if let Some(ref df) = date_filter {
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(duration), 0.0)
             FROM transcriptions WHERE timestamp >= datetime('now', ?1) AND duration > 1.0",
            params![df],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(duration), 0.0)
             FROM transcriptions WHERE duration > 1.0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
    };

    let total_sessions_all: i64 = if let Some(ref df) = date_filter {
        conn.query_row(
            "SELECT COUNT(*) FROM transcriptions WHERE timestamp >= datetime('now', ?1)",
            params![df],
            |row| row.get(0),
        )?
    } else {
        conn.query_row("SELECT COUNT(*) FROM transcriptions", [], |row| row.get(0))?
    };

    let texts: Vec<String> = if let Some(ref df) = date_filter {
        let mut s = conn.prepare(
            "SELECT text FROM transcriptions WHERE timestamp >= datetime('now', ?1)"
        )?;
        let rows = s.query_map(params![df], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut s = conn.prepare("SELECT text FROM transcriptions")?;
        let rows = s.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut total_words: i64 = 0;
    let mut total_keystrokes: i64 = 0;
    let mut total_typing_minutes: f64 = 0.0;
    for text in &texts {
        total_words += count_words(text);
        total_keystrokes += calc_keystrokes(text);
        total_typing_minutes += calc_typing_time_minutes(text);
    }

    let total_recording_minutes = total_duration / 60.0;
    let time_saved = (total_typing_minutes - total_recording_minutes).max(0.0);

    let avg_wpm = if total_duration > 1.0 {
        total_words as f64 / (total_duration / 60.0)
    } else {
        0.0
    };

    let daily_wpm: Vec<DailyWpm> = if let Some(ref df) = date_filter {
        let mut s = conn.prepare(
            "SELECT DATE(timestamp) as day, SUM(word_count) as words, SUM(duration) as dur, COUNT(*) as cnt
             FROM transcriptions
             WHERE timestamp >= datetime('now', ?1) AND duration > 1.0
             GROUP BY day ORDER BY day ASC"
        )?;
        let rows = s.query_map(params![df], |row| {
            let words: f64 = row.get(1)?;
            let dur: f64 = row.get(2)?;
            let wpm = if dur > 0.0 { words / (dur / 60.0) } else { 0.0 };
            Ok(DailyWpm { date: row.get(0)?, wpm, session_count: row.get(3)? })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut s = conn.prepare(
            "SELECT DATE(timestamp) as day, SUM(word_count) as words, SUM(duration) as dur, COUNT(*) as cnt
             FROM transcriptions WHERE duration > 1.0
             GROUP BY day ORDER BY day ASC"
        )?;
        let rows = s.query_map([], |row| {
            let words: f64 = row.get(1)?;
            let dur: f64 = row.get(2)?;
            let wpm = if dur > 0.0 { words / (dur / 60.0) } else { 0.0 };
            Ok(DailyWpm { date: row.get(0)?, wpm, session_count: row.get(3)? })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let wpm_stats = if daily_wpm.is_empty() {
        WpmStats { avg: 0.0, max: 0.0, min: 0.0 }
    } else {
        let sum: f64 = daily_wpm.iter().map(|d| d.wpm).sum();
        let max = daily_wpm.iter().map(|d| d.wpm).fold(0.0f64, f64::max);
        let min = daily_wpm.iter().map(|d| d.wpm).fold(f64::INFINITY, f64::min);
        WpmStats {
            avg: sum / daily_wpm.len() as f64,
            max,
            min: if min.is_infinite() { 0.0 } else { min },
        }
    };

    info!("[db] get_statistics: sessions={} words={} duration={:.1}s keystrokes={} time_saved={:.1}min",
        total_sessions_all, total_words, total_duration, total_keystrokes, time_saved);
    Ok(Statistics {
        total_sessions: total_sessions_all,
        total_words,
        total_duration_seconds: total_duration,
        total_keystrokes_saved: total_keystrokes,
        time_saved_minutes: time_saved,
        avg_wpm,
        daily_wpm,
        wpm_stats,
    })
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

pub fn add_vocabulary(conn: &Connection, word: &str) -> Result<String, AppError> {
    info!("[db] add_vocabulary word={}", word);
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO vocabulary (id, word) VALUES (?1, ?2)",
        params![id, word],
    )
    .map_err(|e| {
        error!("[db] add_vocabulary failed: {}", e);
        e
    })?;
    Ok(id)
}

pub fn get_vocabulary(conn: &Connection) -> Result<Vec<VocabularyWord>, AppError> {
    info!("[db] get_vocabulary");
    let mut stmt =
        conn.prepare("SELECT id, word, created_at FROM vocabulary ORDER BY created_at DESC")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(VocabularyWord {
                id: row.get(0)?,
                word: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    info!("[db] get_vocabulary returned {} items", rows.len());
    Ok(rows)
}

pub fn delete_vocabulary(conn: &Connection, id: &str) -> Result<(), AppError> {
    info!("[db] delete_vocabulary id={}", id);
    conn.execute("DELETE FROM vocabulary WHERE id = ?1", params![id])
        .map_err(|e| {
            error!("[db] delete_vocabulary failed: {}", e);
            e
        })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Replacements
// ---------------------------------------------------------------------------

pub fn set_replacement(
    conn: &Connection,
    original: &str,
    replacement: &str,
) -> Result<String, AppError> {
    info!("[db] set_replacement original={} replacement={}", original, replacement);
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO replacements (id, original, replacement) VALUES (?1, ?2, ?3)",
        params![id, original, replacement],
    )
    .map_err(|e| {
        error!("[db] set_replacement failed: {}", e);
        e
    })?;
    Ok(id)
}

pub fn get_replacements(conn: &Connection) -> Result<Vec<Replacement>, AppError> {
    info!("[db] get_replacements");
    let mut stmt = conn.prepare(
        "SELECT id, original, replacement, created_at FROM replacements ORDER BY created_at DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Replacement {
                id: row.get(0)?,
                original: row.get(1)?,
                replacement: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    info!("[db] get_replacements returned {} items", rows.len());
    Ok(rows)
}

pub fn delete_replacement(conn: &Connection, id: &str) -> Result<(), AppError> {
    info!("[db] delete_replacement id={}", id);
    conn.execute("DELETE FROM replacements WHERE id = ?1", params![id])
        .map_err(|e| {
            error!("[db] delete_replacement failed: {}", e);
            e
        })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

pub fn update_setting(
    conn: &Connection,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), AppError> {
    let is_sensitive = key.contains("api_key") || key.contains("secret");
    if is_sensitive {
        info!("[db] update_setting key={} value={}", key, mask::mask(&serde_json::to_string(value).unwrap_or_default()));
    } else {
        info!("[db] update_setting key={} value={:?}", key, value);
    }
    let json_str = serde_json::to_string(value)
        .map_err(|e| {
            error!("[db] update_setting serialize failed key={}: {}", key, e);
            AppError::InvalidInput(e.to_string())
        })?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json_str],
    )
    .map_err(|e| {
        error!("[db] update_setting failed key={}: {}", key, e);
        e
    })?;
    Ok(())
}

pub fn get_setting(
    conn: &Connection,
    key: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    info!("[db] get_setting key={}", key);
    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;

    match result {
        Some(s) => {
            let val: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| {
                    error!("[db] get_setting deserialize failed key={}: {}", key, e);
                    AppError::Database(e.to_string())
                })?;
            Ok(Some(val))
        }
        None => Ok(None),
    }
}

pub fn get_all_settings(
    conn: &Connection,
) -> Result<HashMap<String, serde_json::Value>, AppError> {
    info!("[db] get_all_settings");
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let val: String = row.get(1)?;
        Ok((key, val))
    })?;

    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r?;
        let parsed: serde_json::Value =
            serde_json::from_str(&v).map_err(|e| {
                error!("[db] get_all_settings deserialize failed key={}: {}", k, e);
                AppError::Database(e.to_string())
            })?;
        map.insert(k, parsed);
    }
    info!("[db] get_all_settings returned {} keys", map.len());
    Ok(map)
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

pub fn add_prompt(
    conn: &Connection,
    name: &str,
    system_message: &str,
    user_message_template: &str,
    is_predefined: bool,
) -> Result<String, AppError> {
    info!("[db] add_prompt name={} is_predefined={}", name, is_predefined);
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO prompts (id, name, system_message, user_message_template, is_predefined)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, system_message, user_message_template, is_predefined as i32],
    )
    .map_err(|e| {
        error!("[db] add_prompt failed: {}", e);
        e
    })?;
    Ok(id)
}

pub fn list_prompts(conn: &Connection) -> Result<Vec<Prompt>, AppError> {
    info!("[db] list_prompts");
    let mut stmt = conn.prepare(
        "SELECT id, name, system_message, user_message_template, is_predefined, created_at
         FROM prompts ORDER BY created_at ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let is_pre: i32 = row.get(4)?;
            Ok(Prompt {
                id: row.get(0)?,
                name: row.get(1)?,
                system_message: row.get(2)?,
                user_message_template: row.get(3)?,
                is_predefined: is_pre != 0,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    info!("[db] list_prompts returned {} items", rows.len());
    Ok(rows)
}

pub fn update_prompt(
    conn: &Connection,
    id: &str,
    name: &str,
    system_message: &str,
    user_message_template: &str,
) -> Result<(), AppError> {
    info!("[db] update_prompt id={} name={}", id, name);
    conn.execute(
        "UPDATE prompts SET name = ?1, system_message = ?2, user_message_template = ?3 WHERE id = ?4",
        params![name, system_message, user_message_template, id],
    )
    .map_err(|e| {
        error!("[db] update_prompt failed: {}", e);
        e
    })?;
    Ok(())
}

pub fn delete_prompt(conn: &Connection, id: &str) -> Result<(), AppError> {
    info!("[db] delete_prompt id={}", id);
    conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])
        .map_err(|e| {
            error!("[db] delete_prompt failed: {}", e);
            e
        })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CSV Import / Export
// ---------------------------------------------------------------------------

pub fn export_vocabulary_csv(conn: &Connection, path: &std::path::Path) -> Result<(), AppError> {
    info!("[db] export_vocabulary_csv path={}", path.display());
    let words = get_vocabulary(conn)?;
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| {
            error!("[db] export_vocabulary_csv open failed: {}", e);
            AppError::Io(e.to_string())
        })?;
    wtr.write_record(&["word"]).map_err(|e| AppError::Io(e.to_string()))?;
    for w in words {
        wtr.write_record(&[&w.word]).map_err(|e| AppError::Io(e.to_string()))?;
    }
    wtr.flush().map_err(|e| AppError::Io(e.to_string()))?;
    info!("[db] export_vocabulary_csv complete");
    Ok(())
}

pub fn import_vocabulary_csv(conn: &Connection, path: &std::path::Path) -> Result<(), AppError> {
    info!("[db] import_vocabulary_csv path={}", path.display());
    let mut rdr = csv::Reader::from_path(path)
        .map_err(|e| {
            error!("[db] import_vocabulary_csv open failed: {}", e);
            AppError::Io(e.to_string())
        })?;
    let mut count = 0usize;
    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Io(e.to_string()))?;
        if let Some(word) = record.get(0) {
            add_vocabulary(conn, word)?;
            count += 1;
        }
    }
    info!("[db] import_vocabulary_csv imported {} words", count);
    Ok(())
}

pub fn export_replacements_csv(conn: &Connection, path: &std::path::Path) -> Result<(), AppError> {
    info!("[db] export_replacements_csv path={}", path.display());
    let reps = get_replacements(conn)?;
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| {
            error!("[db] export_replacements_csv open failed: {}", e);
            AppError::Io(e.to_string())
        })?;
    wtr.write_record(&["original", "replacement"]).map_err(|e| AppError::Io(e.to_string()))?;
    for r in reps {
        wtr.write_record(&[&r.original, &r.replacement]).map_err(|e| AppError::Io(e.to_string()))?;
    }
    wtr.flush().map_err(|e| AppError::Io(e.to_string()))?;
    info!("[db] export_replacements_csv complete");
    Ok(())
}

pub fn import_replacements_csv(conn: &Connection, path: &std::path::Path) -> Result<(), AppError> {
    info!("[db] import_replacements_csv path={}", path.display());
    let mut rdr = csv::Reader::from_path(path)
        .map_err(|e| {
            error!("[db] import_replacements_csv open failed: {}", e);
            AppError::Io(e.to_string())
        })?;
    let mut count = 0usize;
    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Io(e.to_string()))?;
        if let (Some(orig), Some(repl)) = (record.get(0), record.get(1)) {
            set_replacement(conn, orig, repl)?;
            count += 1;
        }
    }
    info!("[db] import_replacements_csv imported {} replacements", count);
    Ok(())
}
