//! Data access layer for the `settings` table.
//!
//! Provides pure CRUD operations with no business logic or caching.

use sqlx::{Pool, Sqlite};

use crate::errors::AppResult;
use crate::models::SettingRow;

/// Settings repository implementation.
pub struct SettingsRepository;

impl SettingsRepository {
    /// Fetches all rows from the `settings` table.
    pub async fn get_all(pool: &Pool<Sqlite>) -> AppResult<Vec<SettingRow>> {
        let rows = sqlx::query_as::<_, SettingRow>(
            "SELECT key, value, value_type, updated_at FROM settings"
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Fetches a single setting row by its unique key.
    pub async fn get_by_key(pool: &Pool<Sqlite>, key: &str) -> AppResult<Option<SettingRow>> {
        let row = sqlx::query_as::<_, SettingRow>(
            "SELECT key, value, value_type, updated_at FROM settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Inserts a new setting key-value pair or updates the value/updated_at if the key already exists.
    pub async fn upsert(
        pool: &Pool<Sqlite>,
        key: &str,
        value: &str,
        value_type: &str,
        updated_at: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO settings (key, value, value_type, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET \
                value = excluded.value, \
                updated_at = excluded.updated_at"
        )
        .bind(key)
        .bind(value)
        .bind(value_type)
        .bind(updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Deletes a setting row by its unique key.
    pub async fn delete_by_key(pool: &Pool<Sqlite>, key: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }
}
