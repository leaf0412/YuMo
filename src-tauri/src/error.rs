use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Recording error: {0}")]
    Recording(String),
    #[error("Transcription error: {0}")]
    Transcription(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Permission denied: {0}")]
    Permission(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
