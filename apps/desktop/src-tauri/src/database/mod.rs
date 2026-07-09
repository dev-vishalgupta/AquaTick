//! SQLite connection pool initializer.
//!
//! `init_db` is the single entry point for all database setup:
//! - Ensures the app data directory exists.
//! - Creates the SQLite file if it does not exist.
//! - Opens a connection pool with production-ready settings.
//! - Runs all pending schema migrations.
//!
//! The returned `Pool<Sqlite>` is stored in `AppState` and shared across
//! all Tauri command handlers for the lifetime of the application.
//!
//! # Lazy initialization
//!
//! The pool is NOT opened until `init_db` is called from `lib.rs::setup`.
//! No database activity occurs before the Tauri application has fully started.

pub mod migrations;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    Pool, Sqlite, SqlitePool,
};
use std::path::Path;

use crate::constants::DEFAULT_DATABASE_NAME;
use crate::errors::AppResult;

/// Opens the SQLite connection pool and runs all pending migrations.
///
/// # Arguments
///
/// * `app_data_dir` — Platform-specific Tauri application data directory,
///   e.g. `%APPDATA%\com.aquatick.app` on Windows.
///   Created automatically if it does not exist.
///
/// # Returns
///
/// A `Pool<Sqlite>` ready for use in repositories and services.
///
/// # Errors
///
/// Returns `AppError::Io` if the directory cannot be created.
/// Returns `AppError::Database` if the connection or pool creation fails.
/// Returns `AppError::Migration` if any migration statement fails.
pub async fn init_db(app_data_dir: &Path) -> AppResult<Pool<Sqlite>> {
    // Create the application data directory if it does not yet exist.
    tokio::fs::create_dir_all(app_data_dir).await?;

    let db_path = app_data_dir.join(DEFAULT_DATABASE_NAME);

    log::info!("Opening database at: {}", db_path.display());

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        // Create the file on first launch; error if missing on subsequent launches
        // would indicate a filesystem problem rather than a normal state.
        .create_if_missing(true)
        // WAL mode allows concurrent readers + one writer without blocking.
        .journal_mode(SqliteJournalMode::Wal)
        // NORMAL synchronous mode — safe with WAL, significantly faster than FULL.
        .synchronous(SqliteSynchronous::Normal)
        // Enforce foreign key constraints at the SQLite level.
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(options).await?;

    log::info!("Database connection pool established.");

    // Run all pending schema migrations (idempotent — safe on every launch).
    migrations::run_migrations(&pool).await?;

    Ok(pool)
}
