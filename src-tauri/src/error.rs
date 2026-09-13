//! Single error type surfaced to the frontend. Every Tauri command returns
//! `Result<T, AppError>`; `AppError` serializes as a plain string so the
//! frontend just sees `err.message` without a schema round-trip.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("waxum: {0}")]
    Waxum(String),
    #[error("elevenlabs: {0}")]
    ElevenLabs(String),
    #[error("bundled binary: {0}")]
    Bundled(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Store(#[from] tauri_plugin_store::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
