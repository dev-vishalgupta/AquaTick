//! Service layer for daily and weekly statistics aggregation.
//!
//! Orchestrates the `StatisticsRepository` and `SessionRepository` to compile daily metrics
//! and weekly rollups.

use sqlx::{Pool, Sqlite};
use chrono::{NaiveDate, Duration};

use crate::errors::{AppError, AppResult};
use crate::models::{DailyStatistic, WeeklyStatistic, SessionStatus};
use crate::repositories::{StatisticsRepository, SessionRepository};

/// Service for managing and aggregating application statistics.
pub struct StatisticsService;

impl StatisticsService {
    /// Fetches the statistics for a specific calendar date (ISO YYYY-MM-DD).
    /// If no statistics row exists yet, returns a zeroed instance.
    pub async fn get_daily_statistics(pool: &Pool<Sqlite>, date: &str) -> AppResult<DailyStatistic> {
        // Simple date format check
        NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            AppError::Validation(format!("Invalid date format '{}'. Expected YYYY-MM-DD", date))
        })?;

        let existing = StatisticsRepository::get_by_date(pool, date).await?;
        match existing {
            Some(stats) => Ok(stats),
            None => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                Ok(DailyStatistic::zero(date, now_ms))
            }
        }
    }

    /// Fetches the aggregated weekly statistics starting on a specific Monday (ISO YYYY-MM-DD).
    pub async fn get_weekly_statistics(pool: &Pool<Sqlite>, week_start: &str) -> AppResult<WeeklyStatistic> {
        let start_date = NaiveDate::parse_from_str(week_start, "%Y-%m-%d").map_err(|_| {
            AppError::Validation(format!(
                "Invalid week_start date format '{}'. Expected YYYY-MM-DD",
                week_start
            ))
        })?;

        // A week always has 7 days. Monday to Sunday.
        let end_date = start_date + Duration::days(6);
        let week_end_str = end_date.format("%Y-%m-%d").to_string();

        let daily_stats = StatisticsRepository::get_range(pool, week_start, &week_end_str).await?;

        let mut total_sessions = 0;
        let mut total_completed = 0;
        let mut days_goal_met = 0;
        let mut longest_streak_in_week = 0;
        let mut sum_completion_pct = 0.0;
        let mut days_with_sessions = 0;

        for day in daily_stats {
            total_sessions += day.total_sessions;
            total_completed += day.completed_count;
            if day.is_goal_met() {
                days_goal_met += 1;
            }
            if day.streak_day > longest_streak_in_week {
                longest_streak_in_week = day.streak_day;
            }
            if day.total_sessions > 0 {
                sum_completion_pct += day.completion_percentage;
                days_with_sessions += 1;
            }
        }

        let average_completion_percentage = if days_with_sessions > 0 {
            sum_completion_pct / days_with_sessions as f64
        } else {
            0.0
        };

        Ok(WeeklyStatistic {
            week_start: week_start.to_string(),
            week_end: week_end_str,
            total_sessions,
            total_completed,
            average_completion_percentage,
            days_goal_met,
            longest_streak_in_week,
        })
    }

    /// Recomputes all daily statistics fields for a specific date and saves the result.
    ///
    /// # Formulas (Architecture §2, §10, §12)
    /// - total_sessions = count of completed or timed_out sessions on date
    /// - completed_count = count of completed sessions on date
    /// - timed_out_count = count of timed_out sessions on date
    /// - snoozed_count = count of sessions on date with snooze_count > 0
    /// - expected_sessions = active_minutes_today / interval_minutes
    /// - goal_met = completed_count >= expected_sessions
    /// - streak_day = consecutive days ending today where goal_met = true
    pub async fn recompute_for_date(
        pool: &Pool<Sqlite>,
        date: &str,
        active_minutes_today: i64,
        interval_minutes: i64,
    ) -> AppResult<()> {
        let date_parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            AppError::Validation(format!("Invalid date format '{}'. Expected YYYY-MM-DD", date))
        })?;

        if interval_minutes <= 0 {
            return Err(AppError::Validation(
                "reminder.interval_minutes must be greater than zero to recompute statistics"
                    .to_string(),
            ));
        }

        // 1. Query all sessions on this calendar date
        let sessions = SessionRepository::get_by_date(pool, date).await?;

        let mut total_sessions = 0;
        let mut completed_count = 0;
        let mut timed_out_count = 0;
        let mut snoozed_count = 0;

        for s in sessions {
            use std::str::FromStr;
            let status = SessionStatus::from_str(&s.status)?;

            if status == SessionStatus::Completed {
                completed_count += 1;
                total_sessions += 1;
            } else if status == SessionStatus::TimedOut {
                timed_out_count += 1;
                total_sessions += 1;
            }

            if s.snooze_count > 0 {
                snoozed_count += 1;
            }
        }

        let completion_percentage = if total_sessions > 0 {
            (completed_count as f64 / total_sessions as f64) * 100.0
        } else {
            0.0
        };

        // expected_sessions = active_minutes_today / interval_minutes
        let expected_sessions = active_minutes_today / interval_minutes;
        let goal_met = if completed_count >= expected_sessions { 1 } else { 0 };

        // Calculate consecutive streak going backward
        // If today's goal is met, streak is yesterday's streak + 1. If not met, streak is 0.
        let yesterday = (date_parsed - Duration::days(1)).format("%Y-%m-%d").to_string();
        let yesterday_streak = StatisticsRepository::get_streak(pool, &yesterday).await.unwrap_or(0);
        let streak_day = if goal_met == 1 { yesterday_streak as i64 + 1 } else { 0 };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let existing = StatisticsRepository::get_by_date(pool, date).await?;

        let stats = DailyStatistic {
            id: existing.as_ref().map(|e| e.id).unwrap_or(0),
            date: date.to_string(),
            total_sessions,
            completed_count,
            snoozed_count,
            timed_out_count,
            completion_percentage,
            expected_sessions,
            goal_met,
            streak_day,
            created_at: existing.as_ref().map(|e| e.created_at).unwrap_or(now_ms),
            updated_at: now_ms,
        };

        StatisticsRepository::upsert(pool, &stats).await?;

        Ok(())
    }
}
