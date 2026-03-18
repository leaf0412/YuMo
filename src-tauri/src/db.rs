use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

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
    let conn = Connection::open(path)?;

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
            word_count INTEGER NOT NULL
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

    seed_predefined_prompts(&conn)?;

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
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    conn.execute(
        "INSERT INTO transcriptions (id, text, enhanced_text, timestamp, duration, model_name, word_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, text, enhanced_text, now, duration, model_name, word_count],
    )?;
    Ok(id)
}

pub fn get_transcriptions(
    conn: &Connection,
    cursor: Option<&str>,
    query: Option<&str>,
    limit: usize,
) -> Result<PaginatedResult, AppError> {
    let items: Vec<TranscriptionRecord> = match (cursor, query) {
        (None, None) => {
            let mut stmt = conn.prepare(
                "SELECT id, text, enhanced_text, timestamp, duration, model_name, word_count
                 FROM transcriptions ORDER BY timestamp DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        (Some(cur), None) => {
            let mut stmt = conn.prepare(
                "SELECT id, text, enhanced_text, timestamp, duration, model_name, word_count
                 FROM transcriptions WHERE timestamp < ?1 ORDER BY timestamp DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![cur, limit as i64], row_to_transcription)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        (None, Some(q)) => {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.text, t.enhanced_text, t.timestamp, t.duration, t.model_name, t.word_count
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
                "SELECT t.id, t.text, t.enhanced_text, t.timestamp, t.duration, t.model_name, t.word_count
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
    })
}

pub fn delete_transcription(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM transcriptions WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_all_transcriptions(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM transcriptions", [])?;
    Ok(())
}

pub fn cleanup_old_transcriptions(conn: &Connection, days: i32) -> Result<usize, AppError> {
    let deleted = conn.execute(
        "DELETE FROM transcriptions WHERE timestamp < datetime('now', ?1)",
        params![format!("-{} days", days)],
    )?;
    Ok(deleted)
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

pub fn add_vocabulary(conn: &Connection, word: &str) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO vocabulary (id, word) VALUES (?1, ?2)",
        params![id, word],
    )?;
    Ok(id)
}

pub fn get_vocabulary(conn: &Connection) -> Result<Vec<VocabularyWord>, AppError> {
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
    Ok(rows)
}

pub fn delete_vocabulary(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM vocabulary WHERE id = ?1", params![id])?;
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
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO replacements (id, original, replacement) VALUES (?1, ?2, ?3)",
        params![id, original, replacement],
    )?;
    Ok(id)
}

pub fn get_replacements(conn: &Connection) -> Result<Vec<Replacement>, AppError> {
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
    Ok(rows)
}

pub fn delete_replacement(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM replacements WHERE id = ?1", params![id])?;
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
    let json_str = serde_json::to_string(value)
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json_str],
    )?;
    Ok(())
}

pub fn get_setting(
    conn: &Connection,
    key: &str,
) -> Result<Option<serde_json::Value>, AppError> {
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
                serde_json::from_str(&s).map_err(|e| AppError::Database(e.to_string()))?;
            Ok(Some(val))
        }
        None => Ok(None),
    }
}

pub fn get_all_settings(
    conn: &Connection,
) -> Result<HashMap<String, serde_json::Value>, AppError> {
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
            serde_json::from_str(&v).map_err(|e| AppError::Database(e.to_string()))?;
        map.insert(k, parsed);
    }
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
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO prompts (id, name, system_message, user_message_template, is_predefined)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, system_message, user_message_template, is_predefined as i32],
    )?;
    Ok(id)
}

pub fn list_prompts(conn: &Connection) -> Result<Vec<Prompt>, AppError> {
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
    Ok(rows)
}

pub fn update_prompt(
    conn: &Connection,
    id: &str,
    name: &str,
    system_message: &str,
    user_message_template: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE prompts SET name = ?1, system_message = ?2, user_message_template = ?3 WHERE id = ?4",
        params![name, system_message, user_message_template, id],
    )?;
    Ok(())
}

pub fn delete_prompt(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
    Ok(())
}
