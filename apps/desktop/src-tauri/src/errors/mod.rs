//! Unified application error type.
//!
//! `AppError` is used throughout all backend modules (database, repositories,
//! services). It is converted to `CommandError` at the IPC boundary so that
//! no raw Rust error strings ever reach the frontend.

use thiserror::Error;

/// All possible errors that can occur in the AquaTick backend.
#[derive(Debug, Error)]
pub enum AppError {
    /// Wraps any `sqlx::Error` — connection, query, or type mapping failures.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Schema migration failures (version mismatch, SQL syntax, etc.).
    #[error("Migration error: {0}")]
    Migration(String),

    /// A required record was not found in the database.
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Input or type validation failure (invalid setting key, bad value type, etc.).
    #[error("Validation error: {0}")]
    Validation(String),

    /// JSON or type-casting failure when deserializing setting values.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Filesystem errors (directory creation, path resolution, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for unexpected internal conditions.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience alias — all backend functions return this.
pub type AppResult<T> = Result<T, AppError>;

// ── IPC boundary ──────────────────────────────────────────────────────────────

/// Serializable error returned to the frontend via IPC.
///
/// All `AppError` variants are mapped to a stable, string `code` so the
/// frontend can match on errors without parsing message strings.
///
/// # Invariant
/// Every `AppError` variant must have exactly one `code` mapping here.
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    /// Stable machine-readable code (e.g. `"NOT_FOUND"`, `"VALIDATION_ERROR"`).
    pub code: String,
    /// Human-readable description of the error.
    pub message: String,
}

impl From<AppError> for CommandError {
    fn from(err: AppError) -> Self {
        let code = match &err {
            AppError::Database(_)      => "DATABASE_ERROR",
            AppError::Migration(_)     => "MIGRATION_ERROR",
            AppError::NotFound(_)      => "NOT_FOUND",
            AppError::Validation(_)    => "VALIDATION_ERROR",
            AppError::Serialization(_) => "SERIALIZATION_ERROR",
            AppError::Io(_)            => "IO_ERROR",
            AppError::Internal(_)      => "INTERNAL_ERROR",
        };
        Self {
            code: code.to_string(),
            message: err.to_string(),
        }
    }
}
