//! Data access layer for the `hydration_sessions` table.
//!
//! Provides pure CRUD operations with no business rules, scheduling, or timer logic.

use sqlx::SqliteConnection;

use crate::errors::AppResult;
use crate::models::{HydrationSession, NewSession, SessionStatus};

/// Session repository implementation.
pub struct SessionRepository;

impl SessionRepository {
    /// Inserts a new hydration session in 'pending' status.
    /// Returns the database-generated auto-incremented ID.
    pub async fn insert(conn: &mut SqliteConnection, session: &NewSession) -> AppResult<i64> {
        let result = sqlx::query(
            "INSERT INTO hydration_sessions ( \
                scheduled_at, triggered_at, responded_at, status, snooze_count, \
                interval_minutes, character_id, sound_id, created_at, updated_at \
             ) VALUES (?, NULL, NULL, 'pending', 0, ?, ?, ?, ?, ?)"
        )
        .bind(session.scheduled_at)
        .bind(session.interval_minutes)
        .bind(&session.character_id)
        .bind(&session.sound_id)
        .bind(session.created_at)
        .bind(session.created_at) // updated_at is initially created_at
        .execute(conn)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Fetches a hydration session by its ID.
    pub async fn get_by_id(conn: &mut SqliteConnection, id: i64) -> AppResult<Option<HydrationSession>> {
        let row = sqlx::query_as::<_, HydrationSession>(
            "SELECT id, scheduled_at, triggered_at, responded_at, status, snooze_count, \
                    interval_minutes, character_id, sound_id, created_at, updated_at \
             FROM hydration_sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(conn)
        .await?;

        Ok(row)
    }

    /// Updates the status, responded_at, and updated_at of a session.
    pub async fn update_status(
        conn: &mut SqliteConnection,
        id: i64,
        status: SessionStatus,
        responded_at: Option<i64>,
        updated_at: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE hydration_sessions \
             SET status = ?, responded_at = ?, updated_at = ? \
             WHERE id = ?"
        )
        .bind(status.as_str())
        .bind(responded_at)
        .bind(updated_at)
        .bind(id)
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Updates the status, triggered_at, and updated_at of a session.
    pub async fn update_triggered(
        conn: &mut SqliteConnection,
        id: i64,
        status: SessionStatus,
        triggered_at: i64,
        updated_at: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE hydration_sessions \
             SET status = ?, triggered_at = ?, updated_at = ? \
             WHERE id = ?"
        )
        .bind(status.as_str())
        .bind(triggered_at)
        .bind(updated_at)
        .bind(id)
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Increments the snooze count and updates updated_at for a session.
    pub async fn increment_snooze(
        conn: &mut SqliteConnection,
        id: i64,
        updated_at: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE hydration_sessions \
             SET snooze_count = snooze_count + 1, updated_at = ? \
             WHERE id = ?"
        )
        .bind(updated_at)
        .bind(id)
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Finds sessions that were left in the 'triggered' state before a specific timestamp.
    /// This is used for crash recovery when the application starts up.
    pub async fn get_triggered_before(
        conn: &mut SqliteConnection,
        timestamp_ms: i64,
    ) -> AppResult<Vec<HydrationSession>> {
        let rows = sqlx::query_as::<_, HydrationSession>(
            "SELECT id, scheduled_at, triggered_at, responded_at, status, snooze_count, \
                    interval_minutes, character_id, sound_id, created_at, updated_at \
             FROM hydration_sessions \
             WHERE status = 'triggered' AND triggered_at < ?"
        )
        .bind(timestamp_ms)
        .fetch_all(conn)
        .await?;

        Ok(rows)
    }

    /// Fetches all sessions scheduled on a specific calendar date (ISO format YYYY-MM-DD).
    pub async fn get_by_date(conn: &mut SqliteConnection, date: &str) -> AppResult<Vec<HydrationSession>> {
        let rows = sqlx::query_as::<_, HydrationSession>(
            "SELECT id, scheduled_at, triggered_at, responded_at, status, snooze_count, \
                    interval_minutes, character_id, sound_id, created_at, updated_at \
             FROM hydration_sessions \
             WHERE date(scheduled_at / 1000, 'unixepoch') = ?"
        )
        .bind(date)
        .fetch_all(conn)
        .await?;

        Ok(rows)
    }
}
