//! Versioned database migration runner.
//!
//! # How migrations work
//!
//! 1. On startup, `run_migrations` checks whether the `settings` table exists.
//! 2. If not, the database is brand-new → current version is `0`.
//! 3. If it exists, the current version is read from the `db.schema_version` key.
//! 4. Every `Migration` whose `version` exceeds the current version is applied
//!    in ascending order.
//! 5. After each migration, `db.schema_version` is updated in `settings`.
//!
//! # Adding a future migration
//!
//! 1. Add a new `const MIGRATION_VN: &[&str]` array with the SQL statements.
//! 2. Append a `Migration { version: N, ... }` entry to `MIGRATIONS`.
//! 3. Increment `constants::SCHEMA_VERSION` to `N`.
//!
//! # Safety
//!
//! - Every statement uses `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`
//!   so re-running a migration on an existing schema is harmless.
//! - `INSERT OR IGNORE` ensures seed data is never duplicated.
//! - Each migration executes inside its own database transaction.

use sqlx::{Pool, Sqlite};

use crate::constants::{self, setting_keys, APP_VERSION, DEFAULT_CHARACTER_ID};
use crate::errors::{AppError, AppResult};

// ─────────────────────────────────────────────────────────────────────────────
// Migration definitions
// ─────────────────────────────────────────────────────────────────────────────

/// Ordered list of SQL statements for Migration V1 — the initial schema.
const MIGRATION_V1: &[&str] = &[
    // Settings table
    r#"CREATE TABLE IF NOT EXISTS settings (
        key         TEXT    PRIMARY KEY NOT NULL,
        value       TEXT    NOT NULL,
        value_type  TEXT    NOT NULL
                    CHECK(value_type IN ('string','integer','real','boolean','json')),
        updated_at  INTEGER NOT NULL
    )"#,

    // Hydration sessions table
    r#"CREATE TABLE IF NOT EXISTS hydration_sessions (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        scheduled_at     INTEGER NOT NULL,
        triggered_at     INTEGER,
        responded_at     INTEGER,
        status           TEXT    NOT NULL DEFAULT 'pending'
                         CHECK(status IN ('pending','triggered','completed','snoozed','timed_out')),
        snooze_count     INTEGER NOT NULL DEFAULT 0,
        interval_minutes INTEGER NOT NULL,
        character_id     TEXT    NOT NULL,
        sound_id         TEXT,
        created_at       INTEGER NOT NULL,
        updated_at       INTEGER NOT NULL
    )"#,

    r#"CREATE INDEX IF NOT EXISTS idx_hydration_sessions_scheduled_at
        ON hydration_sessions(scheduled_at)"#,

    r#"CREATE INDEX IF NOT EXISTS idx_hydration_sessions_status
        ON hydration_sessions(status)"#,

    // Daily statistics table
    r#"CREATE TABLE IF NOT EXISTS daily_statistics (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        date                  TEXT    NOT NULL UNIQUE,
        total_sessions        INTEGER NOT NULL DEFAULT 0,
        completed_count       INTEGER NOT NULL DEFAULT 0,
        snoozed_count         INTEGER NOT NULL DEFAULT 0,
        timed_out_count       INTEGER NOT NULL DEFAULT 0,
        completion_percentage REAL    NOT NULL DEFAULT 0.0,
        expected_sessions     INTEGER NOT NULL DEFAULT 0,
        goal_met              INTEGER NOT NULL DEFAULT 0,
        streak_day            INTEGER NOT NULL DEFAULT 0,
        created_at            INTEGER NOT NULL,
        updated_at            INTEGER NOT NULL
    )"#,

    r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_daily_statistics_date
        ON daily_statistics(date)"#,
];

/// Registered migration list.
/// INVARIANT: versions must be unique and in strictly ascending order.
struct Migration {
    version: i64,
    description: &'static str,
    statements: &'static [&'static str],
}

const MIGRATION_V2: &[&str] = &[
    "UPDATE settings SET value = 'female', updated_at = (strftime('%s','now')*1000) WHERE key = 'character.id' AND value = 'female_default'",
    "UPDATE hydration_sessions SET character_id = 'female', updated_at = (strftime('%s','now')*1000) WHERE character_id = 'female_default'",
];

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema: settings, hydration_sessions, daily_statistics",
        statements: MIGRATION_V1,
    },
    Migration {
        version: 2,
        description: "Migrate default character ID from female_default to female",
        statements: MIGRATION_V2,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `true` if the `settings` table exists in `sqlite_master`.
async fn settings_table_exists(pool: &Pool<Sqlite>) -> AppResult<bool> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='settings'",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}

/// Reads the current schema version from the `settings` table.
/// Returns `0` if the key is absent (fresh database before V1 seeding).
async fn current_schema_version(pool: &Pool<Sqlite>) -> AppResult<i64> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(setting_keys::DB_SCHEMA_VERSION)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((v,)) => v.parse::<i64>().map_err(|_| {
            AppError::Migration(
                "db.schema_version is not a valid integer — the settings table may be corrupted"
                    .to_string(),
            )
        }),
        None => Ok(0),
    }
}

/// Seeds the seven default rows into the `settings` table.
/// Uses `INSERT OR IGNORE` — existing rows are never overwritten.
async fn seed_defaults(pool: &Pool<Sqlite>, now_ms: i64) -> AppResult<()> {
    use constants::*;

    let defaults: &[(&str, &str, &str)] = &[
        (setting_keys::DB_SCHEMA_VERSION,       "1",                    "integer"),
        (setting_keys::APP_VERSION,             APP_VERSION,            "string"),
        (setting_keys::REMINDER_INTERVAL_MINUTES, &DEFAULT_REMINDER_INTERVAL_MINUTES.to_string(), "integer"),
        (setting_keys::REMINDER_ENABLED,        "true",                 "boolean"),
        (setting_keys::REMINDER_SOUND_ENABLED,  "true",                 "boolean"),
        (setting_keys::REMINDER_SOUND_VOLUME,   &DEFAULT_SOUND_VOLUME.to_string(), "real"),
        (setting_keys::CHARACTER_ID,            DEFAULT_CHARACTER_ID,   "string"),
    ];

    for (key, value, value_type) in defaults {
        sqlx::query(
            "INSERT OR IGNORE INTO settings (key, value, value_type, updated_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(key)
        .bind(value)
        .bind(value_type)
        .bind(now_ms)
        .execute(pool)
        .await?;
    }

    log::debug!("Default settings seeded (INSERT OR IGNORE — existing rows unchanged).");
    Ok(())
}

/// Applies a single migration's statements inside a database transaction,
/// then seeds defaults (V1 only) and updates `db.schema_version`.
async fn apply_migration(pool: &Pool<Sqlite>, migration: &Migration) -> AppResult<()> {
    log::info!(
        "Applying migration v{}: {}",
        migration.version,
        migration.description
    );

    let mut tx = pool.begin().await?;

    for statement in migration.statements {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::Migration(format!(
                    "Migration v{} statement failed: {}\nSQL: {}",
                    migration.version, e, statement
                ))
            })?;
    }

    tx.commit().await?;

    // Seed default settings after V1 creates the `settings` table.
    if migration.version == 1 {
        let now_ms = chrono::Utc::now().timestamp_millis();
        seed_defaults(pool, now_ms).await?;
    }

    // Record the new version in `settings`.
    let now_ms = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "UPDATE settings SET value = ?, updated_at = ? WHERE key = ?",
    )
    .bind(migration.version.to_string())
    .bind(now_ms)
    .bind(setting_keys::DB_SCHEMA_VERSION)
    .execute(pool)
    .await?;

    log::info!("Migration v{} applied successfully.", migration.version);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Runs all pending migrations in ascending version order.
///
/// Safe to call on every application startup — already-applied migrations
/// are skipped. If no migrations are pending, logs and returns immediately.
pub async fn run_migrations(pool: &Pool<Sqlite>) -> AppResult<()> {
    let table_exists = settings_table_exists(pool).await?;
    let current_version = if table_exists {
        current_schema_version(pool).await?
    } else {
        0
    };

    log::info!("Current database schema version: {}", current_version);

    let mut applied = 0_u32;

    for migration in MIGRATIONS {
        if migration.version > current_version {
            apply_migration(pool, migration).await?;
            applied += 1;
        }
    }

    if applied == 0 {
        log::info!(
            "Database schema is up to date (v{}).",
            current_version
        );
    } else {
        log::info!(
            "{} migration(s) applied. Schema is now at v{}.",
            applied,
            constants::SCHEMA_VERSION
        );
    }

    Ok(())
}
