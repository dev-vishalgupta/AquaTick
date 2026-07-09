//! Data models for the `daily_statistics` table and derived weekly aggregates.
//!
//! `DailyStatistic` mirrors the raw DB row.
//! `WeeklyStatistic` is computed by `StatisticsService` from a 7-day window
//! of `DailyStatistic` rows and is never stored in the database.

use serde::{Deserialize, Serialize};

// ── Daily statistic ───────────────────────────────────────────────────────────

/// Mirrors a single row in the `daily_statistics` table.
///
/// # Invariants (Architecture §12, State Machine §8)
/// - `completion_percentage` is always derived: `(completed_count / total_sessions) * 100`.
/// - `expected_sessions` is always derived: `active_minutes_today / interval_minutes`.
/// - `goal_met` is always derived: `completed_count >= expected_sessions`.
/// - These fields are NEVER set directly from the frontend.
/// - All derivation runs in `StatisticsService::recompute_for_date` in Rust.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DailyStatistic {
    pub id: i64,
    /// ISO-8601 calendar date: `"YYYY-MM-DD"`.
    pub date: String,
    /// Total sessions that reached a final status (`completed` or `timed_out`).
    pub total_sessions: i64,
    /// Sessions where the user confirmed drinking.
    pub completed_count: i64,
    /// Sessions snoozed at least once before final resolution.
    pub snoozed_count: i64,
    /// Sessions that expired without any user response.
    pub timed_out_count: i64,
    /// `(completed_count / total_sessions) * 100.0`. `0.0` when `total_sessions == 0`.
    pub completion_percentage: f64,
    /// Auto-calculated: `active_minutes_today / interval_minutes`. Never user-input.
    pub expected_sessions: i64,
    /// `1` if `completed_count >= expected_sessions`, else `0`. SQLite boolean.
    pub goal_met: i64,
    /// Consecutive day number in the current completion streak ending this date.
    pub streak_day: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl DailyStatistic {
    /// Creates a zero-value statistic for a date that has no sessions yet.
    ///
    /// Used when `statistics_repo::get_by_date` returns `None` and the caller
    /// still needs a valid struct (e.g. for today's initial display).
    pub fn zero(date: &str, now_ms: i64) -> Self {
        Self {
            id: 0,
            date: date.to_string(),
            total_sessions: 0,
            completed_count: 0,
            snoozed_count: 0,
            timed_out_count: 0,
            completion_percentage: 0.0,
            expected_sessions: 0,
            goal_met: 0,
            streak_day: 0,
            created_at: now_ms,
            updated_at: now_ms,
        }
    }

    /// Returns `true` if the daily completion goal was met.
    pub fn is_goal_met(&self) -> bool {
        self.goal_met != 0
    }

    /// Returns the completion percentage as an `f64` clamped to `[0.0, 100.0]`.
    pub fn completion_pct(&self) -> f64 {
        self.completion_percentage.clamp(0.0, 100.0)
    }
}

// ── Weekly statistic ──────────────────────────────────────────────────────────

/// Derived weekly summary computed from a 7-day window of `DailyStatistic` rows.
///
/// This struct is never stored in the database. It is computed by
/// `StatisticsService::get_week` and returned directly as an IPC payload.
///
/// # Invariant ST-5
/// React never calculates weekly statistics. All aggregation runs in Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyStatistic {
    /// ISO-8601 date of Monday (first day of the window): `"YYYY-MM-DD"`.
    pub week_start: String,
    /// ISO-8601 date of Sunday (last day of the window): `"YYYY-MM-DD"`.
    pub week_end: String,
    /// Sum of `total_sessions` across the 7 days.
    pub total_sessions: i64,
    /// Sum of `completed_count` across the 7 days.
    pub total_completed: i64,
    /// Mean of `completion_percentage` across days that had at least one session.
    pub average_completion_percentage: f64,
    /// Number of days (0–7) where `goal_met = true`.
    pub days_goal_met: i64,
    /// Longest consecutive `streak_day` value within the week.
    pub longest_streak_in_week: i64,
}
