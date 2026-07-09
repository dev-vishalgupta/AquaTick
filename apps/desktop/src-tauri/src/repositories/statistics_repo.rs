//! Data access layer for the `daily_statistics` table.
//!
//! Provides pure CRUD operations with no business rules or statistics calculation logic.

use sqlx::{Pool, Sqlite};

use crate::errors::AppResult;
use crate::models::DailyStatistic;

/// Statistics repository implementation.
pub struct StatisticsRepository;

impl StatisticsRepository {
    /// Fetches the daily statistic row for a specific calendar date (ISO format YYYY-MM-DD).
    pub async fn get_by_date(pool: &Pool<Sqlite>, date: &str) -> AppResult<Option<DailyStatistic>> {
        let row = sqlx::query_as::<_, DailyStatistic>(
            "SELECT id, date, total_sessions, completed_count, snoozed_count, timed_out_count, \
                    completion_percentage, expected_sessions, goal_met, streak_day, created_at, updated_at \
             FROM daily_statistics WHERE date = ?"
        )
        .bind(date)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Fetches daily statistics for a range of dates (inclusive).
    /// Dates are in ISO format YYYY-MM-DD.
    pub async fn get_range(
        pool: &Pool<Sqlite>,
        start_date: &str,
        end_date: &str,
    ) -> AppResult<Vec<DailyStatistic>> {
        let rows = sqlx::query_as::<_, DailyStatistic>(
            "SELECT id, date, total_sessions, completed_count, snoozed_count, timed_out_count, \
                    completion_percentage, expected_sessions, goal_met, streak_day, created_at, updated_at \
             FROM daily_statistics \
             WHERE date >= ? AND date <= ? \
             ORDER BY date ASC"
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Inserts a daily statistic row or updates all its metrics if the date already exists.
    pub async fn upsert(pool: &Pool<Sqlite>, stats: &DailyStatistic) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO daily_statistics ( \
                date, total_sessions, completed_count, snoozed_count, timed_out_count, \
                completion_percentage, expected_sessions, goal_met, streak_day, created_at, updated_at \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(date) DO UPDATE SET \
                total_sessions = excluded.total_sessions, \
                completed_count = excluded.completed_count, \
                snoozed_count = excluded.snoozed_count, \
                timed_out_count = excluded.timed_out_count, \
                completion_percentage = excluded.completion_percentage, \
                expected_sessions = excluded.expected_sessions, \
                goal_met = excluded.goal_met, \
                streak_day = excluded.streak_day, \
                updated_at = excluded.updated_at"
        )
        .bind(&stats.date)
        .bind(stats.total_sessions)
        .bind(stats.completed_count)
        .bind(stats.snoozed_count)
        .bind(stats.timed_out_count)
        .bind(stats.completion_percentage)
        .bind(stats.expected_sessions)
        .bind(stats.goal_met)
        .bind(stats.streak_day)
        .bind(stats.created_at)
        .bind(stats.updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Calculates the current consecutive daily streak ending on the given date (inclusive).
    ///
    /// It queries backward day-by-day using a recursive Common Table Expression (CTE).
    pub async fn get_streak(pool: &Pool<Sqlite>, date: &str) -> AppResult<i32> {
        let row: (i64,) = sqlx::query_as(
            "WITH RECURSIVE streak(d, met) AS ( \
                SELECT date, goal_met \
                FROM daily_statistics \
                WHERE date = ? \
                \
                UNION ALL \
                \
                SELECT ds.date, ds.goal_met \
                FROM daily_statistics ds \
                JOIN streak ON ds.date = date(streak.d, '-1 day') \
                WHERE streak.met = 1 AND ds.goal_met = 1 \
             ) \
             SELECT COUNT(*) FROM streak WHERE met = 1"
        )
        .bind(date)
        .fetch_one(pool)
        .await?;

        Ok(row.0 as i32)
    }
}
