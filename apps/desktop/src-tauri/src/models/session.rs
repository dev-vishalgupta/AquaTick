//! Data models for the `hydration_sessions` table.
//!
//! `SessionStatus` is the state machine value stored in the `status` column.
//! `HydrationSession` mirrors the raw DB row.
//! `NewSession` is the input struct for creating a new session row.

use serde::{Deserialize, Serialize};

// ── Status enum ───────────────────────────────────────────────────────────────

/// Valid values for the `hydration_sessions.status` column.
///
/// Mirrors the CHECK constraint:
///   `CHECK(status IN ('pending','triggered','completed','snoozed','timed_out'))`
///
/// State machine transitions (Architecture §2, State Machine §2):
///   pending → triggered → completed
///                       → timed_out
///                       → snoozed → triggered (loop, snooze_count++)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Pending,
    Triggered,
    Completed,
    Snoozed,
    TimedOut,
}

impl SessionStatus {
    /// Returns the exact string stored in the `status` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending   => "pending",
            Self::Triggered => "triggered",
            Self::Completed => "completed",
            Self::Snoozed   => "snoozed",
            Self::TimedOut  => "timed_out",
        }
    }

    /// Returns `true` if this status is a final, immutable terminal state.
    ///
    /// # Invariant S-2
    /// A session in a final state must never change status again.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Completed | Self::TimedOut)
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = crate::errors::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending"   => Ok(Self::Pending),
            "triggered" => Ok(Self::Triggered),
            "completed" => Ok(Self::Completed),
            "snoozed"   => Ok(Self::Snoozed),
            "timed_out" => Ok(Self::TimedOut),
            other => Err(crate::errors::AppError::Validation(format!(
                "Unknown session status: '{}'. \
                 Expected: pending | triggered | completed | snoozed | timed_out",
                other
            ))),
        }
    }
}

// ── Raw DB row ────────────────────────────────────────────────────────────────

/// Mirrors a single row in the `hydration_sessions` table.
///
/// `status` is stored as a raw `String`. Parse it with `SessionStatus::from_str`
/// in the service layer; do not parse it in repositories.
///
/// Timestamps (`scheduled_at`, `triggered_at`, `responded_at`, `created_at`,
/// `updated_at`) are Unix milliseconds stored as `INTEGER`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HydrationSession {
    pub id: i64,
    /// Unix ms when this session was scheduled to fire.
    pub scheduled_at: i64,
    /// Unix ms when the character appeared. `None` until triggered.
    pub triggered_at: Option<i64>,
    /// Unix ms when the user completed / session timed out. `None` until resolved.
    pub responded_at: Option<i64>,
    /// Raw status string — use `SessionStatus::from_str` to parse.
    pub status: String,
    /// Total number of snoozes before the final resolution.
    pub snooze_count: i64,
    /// Snapshot of `reminder.interval_minutes` at scheduling time.
    pub interval_minutes: i64,
    /// Which character was displayed.
    pub character_id: String,
    /// Which sound played. `None` if sound was disabled.
    pub sound_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ── Input struct ──────────────────────────────────────────────────────────────

/// Input for creating a new `hydration_sessions` row.
///
/// Used by `SessionRepository::insert` and `SessionService::create_pending`.
/// The `id`, `triggered_at`, `responded_at`, `status`, and `snooze_count`
/// fields are set by the database / service layer and must not appear here.
#[derive(Debug, Clone)]
pub struct NewSession {
    /// Unix ms when this session is scheduled to fire.
    pub scheduled_at: i64,
    /// Snapshot of `reminder.interval_minutes` at this moment.
    pub interval_minutes: i64,
    /// Character manifest ID from current settings.
    pub character_id: String,
    /// Sound ID from current settings. `None` if sound disabled.
    pub sound_id: Option<String>,
    pub created_at: i64,
}
