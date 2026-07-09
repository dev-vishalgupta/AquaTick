//! Service layer for hydration sessions.
//!
//! Orchestrates the `SessionRepository` and enforces session status transition rules.

use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::errors::{AppError, AppResult};
use crate::models::{HydrationSession, NewSession, SessionStatus};
use crate::repositories::SessionRepository;

/// Service for managing hydration sessions.
pub struct SessionService;

impl SessionService {
    /// Creates a new hydration session in the 'pending' status.
    pub async fn create_pending(
        pool: &Pool<Sqlite>,
        scheduled_at: i64,
        interval_minutes: i64,
        character_id: String,
        sound_id: Option<String>,
    ) -> AppResult<HydrationSession> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let new_session = NewSession {
            scheduled_at,
            interval_minutes,
            character_id,
            sound_id,
            created_at: now_ms,
        };

        let id = SessionRepository::insert(pool, &new_session).await?;
        let session = Self::get_by_id(pool, id).await?;
        Ok(session)
    }

    /// Fetches a hydration session by its unique ID.
    pub async fn get_by_id(pool: &Pool<Sqlite>, id: i64) -> AppResult<HydrationSession> {
        SessionRepository::get_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Hydration session ID {} not found", id)))
    }

    /// Transitions a session's status to 'triggered' and logs the triggered_at timestamp.
    pub async fn mark_triggered(pool: &Pool<Sqlite>, id: i64) -> AppResult<()> {
        let session = Self::get_by_id(pool, id).await?;
        let current_status = SessionStatus::from_str(&session.status)?;
        let target_status = SessionStatus::Triggered;

        Self::validate_transition(&current_status, &target_status)?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        SessionRepository::update_triggered(pool, id, target_status, now_ms, now_ms).await?;

        Ok(())
    }

    /// Transitions a session to 'completed' status.
    pub async fn complete(pool: &Pool<Sqlite>, id: i64) -> AppResult<()> {
        let session = Self::get_by_id(pool, id).await?;
        let current_status = SessionStatus::from_str(&session.status)?;
        let target_status = SessionStatus::Completed;

        Self::validate_transition(&current_status, &target_status)?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        SessionRepository::update_status(pool, id, target_status, Some(now_ms), now_ms).await?;

        Ok(())
    }

    /// Transitions a session to 'timed_out' status.
    pub async fn mark_timed_out(pool: &Pool<Sqlite>, id: i64) -> AppResult<()> {
        let session = Self::get_by_id(pool, id).await?;
        let current_status = SessionStatus::from_str(&session.status)?;
        let target_status = SessionStatus::TimedOut;

        Self::validate_transition(&current_status, &target_status)?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        SessionRepository::update_status(pool, id, target_status, Some(now_ms), now_ms).await?;

        Ok(())
    }

    /// Transitions a session to 'snoozed' status and increments the snooze count.
    pub async fn snooze(pool: &Pool<Sqlite>, id: i64) -> AppResult<()> {
        let session = Self::get_by_id(pool, id).await?;
        let current_status = SessionStatus::from_str(&session.status)?;
        let target_status = SessionStatus::Snoozed;

        Self::validate_transition(&current_status, &target_status)?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        // Update status to 'snoozed' (responded_at is NULL during snooze)
        SessionRepository::update_status(pool, id, target_status, None, now_ms).await?;
        SessionRepository::increment_snooze(pool, id, now_ms).await?;

        Ok(())
    }

    /// Recovers triggered sessions that were left hanging (e.g. due to crash).
    /// Transitions them to 'timed_out' if they exceeded the timeout.
    pub async fn recover_stale(
        pool: &Pool<Sqlite>,
        current_time_ms: i64,
        timeout_minutes: i64,
    ) -> AppResult<u32> {
        let timeout_ms = timeout_minutes * 60 * 1000;
        let cutoff_time = current_time_ms - timeout_ms;

        let stale_sessions = SessionRepository::get_triggered_before(pool, cutoff_time).await?;
        let count = stale_sessions.len() as u32;

        for session in stale_sessions {
            // Force transition to timed_out (bypassing normal validation because it is recovery)
            SessionRepository::update_status(
                pool,
                session.id,
                SessionStatus::TimedOut,
                Some(current_time_ms),
                current_time_ms,
            )
            .await?;
        }

        Ok(count)
    }

    // ── Helper functions ────────────────────────────────────────────────────────

    /// Enforces state machine transition rules.
    ///
    /// Rules (State Machine §2):
    /// - `pending` -> `triggered`
    /// - `triggered` -> `completed`, `timed_out`, `snoozed`
    /// - `snoozed` -> `triggered`
    fn validate_transition(current: &SessionStatus, target: &SessionStatus) -> AppResult<()> {
        if current.is_final() {
            return Err(AppError::Validation(format!(
                "Cannot transition from final status '{}' for session",
                current
            )));
        }

        let is_valid = match current {
            SessionStatus::Pending => matches!(target, SessionStatus::Triggered),
            SessionStatus::Triggered => matches!(
                target,
                SessionStatus::Completed | SessionStatus::TimedOut | SessionStatus::Snoozed
            ),
            SessionStatus::Snoozed => matches!(target, SessionStatus::Triggered),
            _ => false,
        };

        if !is_valid {
            return Err(AppError::Validation(format!(
                "Invalid status transition from '{}' to '{}'",
                current, target
            )));
        }

        Ok(())
    }
}
