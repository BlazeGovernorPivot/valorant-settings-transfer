//! Application error type shared by every module.
//!
//! Tauri commands must return errors that implement `Serialize`, so the
//! error is flattened into its display string when it crosses the IPC bridge.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid Riot ID \"{0}\" (expected Name#TAG)")]
    InvalidRiotId(String),

    #[error("crosshair code error: {0}")]
    Crosshair(String),

    #[error("transfer rejected: {0}")]
    Transfer(String),

    #[error("Riot client: {0}")]
    Riot(String),

    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

pub type AppResult<T> = Result<T, AppError>;

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}
