//! Service layer for daily and weekly statistics aggregation.
//!
//! Orchestrates the `StatisticsRepository` and `SessionRepository` to compile daily metrics
//! and weekly rollups.

use sqlx::{Pool, Sqlite};
use chrono::{NaiveDate, Duration};

use crate::errors::{AppError, AppResult};
use crate::models::{
    DailyStatistic, WeeklyStatistic, SessionStatus,
    DashboardRange, DashboardResponseDTO, DashboardSummaryDTO, ChartDataPointDTO, StreakDataDTO, HeatmapDayDTO
};
use chrono::Datelike;
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

        let mut conn = pool.acquire().await?;
        let existing = StatisticsRepository::get_by_date(&mut conn, date).await?;
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

        let mut conn = pool.acquire().await?;
        let daily_stats = StatisticsRepository::get_range(&mut conn, week_start, &week_end_str).await?;

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
    /// - total_sessions = count of completed, timed_out, or ignored sessions on date
    /// - completed_count = count of completed sessions on date
    /// - timed_out_count = count of timed_out sessions on date
    /// - ignored_count = count of ignored sessions on date
    /// - snoozed_count = count of sessions on date with snooze_count > 0
    /// - expected_sessions = active_minutes_today / interval_minutes
    /// - goal_met = completed_count >= expected_sessions
    /// - streak_day = consecutive days ending today where goal_met = true
    pub async fn recompute_for_date(
        conn: &mut sqlx::SqliteConnection,
        date: &str,
        active_ms_today: i64,
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
        let sessions = SessionRepository::get_by_date(conn, date).await?;

        let mut total_sessions = 0;
        let mut completed_count = 0;
        let mut timed_out_count = 0;
        let mut ignored_count = 0;
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
            } else if status == SessionStatus::Ignored {
                ignored_count += 1;
                total_sessions += 1;
            }

            if s.snooze_count > 0 {
                snoozed_count += 1;
            }
        }

        // Fetch settings for target mode and manual target calculation
        let rows = crate::repositories::SettingsRepository::get_all(conn).await?;
        let mut target_mode = "automatic".to_string();
        let mut manual_target = 8;
        for row in rows {
            match row.key.as_str() {
                crate::constants::setting_keys::TARGET_MODE => {
                    target_mode = row.value;
                }
                crate::constants::setting_keys::MANUAL_TARGET_REMINDERS => {
                    if let Ok(v) = row.value.parse::<i64>() { manual_target = v; }
                }
                _ => {}
            }
        }

        let expected_sessions = if target_mode == "automatic" {
            let active_minutes = active_ms_today / 60000;
            if active_minutes > 0 {
                std::cmp::max(1, active_minutes / interval_minutes)
            } else {
                0
            }
        } else {
            manual_target
        };

        let completion_percentage = if expected_sessions > 0 {
            (completed_count as f64 / expected_sessions as f64) * 100.0
        } else {
            0.0
        };

        let goal_met = if expected_sessions > 0 && completed_count >= expected_sessions { 1 } else { 0 };

        // Calculate consecutive streak going backward
        // If today's goal is met, streak is yesterday's streak + 1. If not met, streak is 0.
        let yesterday = (date_parsed - Duration::days(1)).format("%Y-%m-%d").to_string();
        let yesterday_streak = StatisticsRepository::get_streak(conn, &yesterday).await.unwrap_or(0);
        let streak_day = if goal_met == 1 { yesterday_streak as i64 + 1 } else { 0 };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let existing = StatisticsRepository::get_by_date(conn, date).await?;

        let stats = DailyStatistic {
            id: existing.as_ref().map(|e| e.id).unwrap_or(0),
            date: date.to_string(),
            total_sessions,
            completed_count,
            snoozed_count,
            timed_out_count,
            ignored_count,
            completion_percentage,
            expected_sessions,
            goal_met,
            streak_day,
            working_time_ms: if active_ms_today >= 0 { active_ms_today } else { existing.as_ref().map(|e| e.working_time_ms).unwrap_or(0) },
            created_at: existing.as_ref().map(|e| e.created_at).unwrap_or(now_ms),
            updated_at: now_ms,
        };

        StatisticsRepository::upsert(conn, &stats).await?;

        Ok(())
    }

    /// Generates the complete Statistics Dashboard data.
    ///
    /// This is a strictly read-only operation. It MUST NOT:
    /// - modify SQLite
    /// - update statistics
    /// - update settings
    /// - write caches
    /// - trigger side effects
    pub async fn generate_dashboard(
        pool: &Pool<Sqlite>,
        range: DashboardRange,
        active_ms_today: u64,
    ) -> AppResult<DashboardResponseDTO> {
        let mut conn = pool.acquire().await?;

        // 1. load settings
        let rows = crate::repositories::SettingsRepository::get_all(&mut conn).await?;
        let mut interval = 60;
        let mut target_mode = "automatic".to_string();
        let mut manual_target = 8;
        for row in rows {
            match row.key.as_str() {
                crate::constants::setting_keys::REMINDER_INTERVAL_MINUTES => {
                    if let Ok(v) = row.value.parse::<i64>() { interval = v; }
                }
                crate::constants::setting_keys::TARGET_MODE => {
                    target_mode = row.value;
                }
                crate::constants::setting_keys::MANUAL_TARGET_REMINDERS => {
                    if let Ok(v) = row.value.parse::<i64>() { manual_target = v; }
                }
                _ => {}
            }
        }

        // 2. load session/statistics data
        let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let today_stats = Self::compute_today_stats(
            &mut conn,
            &today_str,
            active_ms_today,
            &target_mode,
            manual_target,
            interval,
        )
        .await?;

        // Get past statistics for streak and heatmap
        let db_stats = StatisticsRepository::get_range(&mut conn, "1970-01-01", &today_str).await?;
        
        let mut all_stats = db_stats;
        if let Some(pos) = all_stats.iter().position(|s| s.date == today_str) {
            all_stats[pos] = today_stats.clone();
        } else {
            all_stats.push(today_stats.clone());
        }
        all_stats.sort_by(|a, b| a.date.cmp(&b.date));

        // 3. build components
        let summary = Self::build_summary(&today_stats, active_ms_today, interval, &target_mode)?;
        let chart = Self::build_chart(&mut conn, &range, &today_stats).await?;
        let heatmap = Self::build_heatmap(&mut conn, &today_stats).await?;
        let streak = Self::build_streak(&all_stats, &today_stats, active_ms_today)?;

        // 4. return DashboardResponseDTO
        Ok(DashboardResponseDTO {
            summary,
            chart,
            heatmap,
            streak,
        })
    }

    // ── Internal Helpers ──────────────────────────────────────────────────────

    fn build_summary(
        today_stats: &DailyStatistic,
        active_ms_today: u64,
        interval: i64,
        target_mode: &str,
    ) -> AppResult<DashboardSummaryDTO> {
        Ok(DashboardSummaryDTO {
            completion_rate: today_stats.completion_pct(),
            completed_reminders: today_stats.completed_count,
            target_reminders: today_stats.expected_sessions,
            snoozed_reminders: today_stats.snoozed_count,
            ignored_reminders: today_stats.timed_out_count + today_stats.ignored_count,
            working_time_ms: active_ms_today as i64,
            reminder_interval: interval,
            target_mode: target_mode.to_string(),
        })
    }

    async fn build_chart(
        conn: &mut sqlx::SqliteConnection,
        range: &DashboardRange,
        today_stats: &DailyStatistic,
    ) -> AppResult<Vec<ChartDataPointDTO>> {
        let mut points = Vec::new();
        match range {
            DashboardRange::Daily => {
                let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let sessions = SessionRepository::get_by_date(conn, &today_str).await?;
                
                let mut sorted_sessions = sessions;
                sorted_sessions.sort_by_key(|s| s.triggered_at.unwrap_or(s.scheduled_at));
                
                for s in sorted_sessions {
                    use std::str::FromStr;
                    let status = SessionStatus::from_str(&s.status)?;
                    let trigger_ms = s.triggered_at.unwrap_or(s.scheduled_at);
                    
                    use chrono::{TimeZone, Local};
                    let dt = Local.timestamp_millis_opt(trigger_ms).unwrap();
                    let label = dt.format("%I:%M %p").to_string();
                    
                    let completed = if status == SessionStatus::Completed { 1 } else { 0 };
                    let ignored = if status == SessionStatus::TimedOut || status == SessionStatus::Ignored { 1 } else { 0 };
                    let snoozed = s.snooze_count;
                    let target = 1;
                    let completion_percentage = if completed == 1 { 100.0 } else { 0.0 };
                    
                    points.push(ChartDataPointDTO {
                        label,
                        completion_percentage,
                        completed,
                        target,
                        snoozed,
                        ignored,
                    });
                }
            }
            DashboardRange::Weekly => {
                let today = chrono::Utc::now().date_naive();
                for i in (0..7).rev() {
                    let date = today - chrono::Duration::days(i);
                    let date_str = date.format("%Y-%m-%d").to_string();
                    
                    let stats = if date == today {
                        today_stats.clone()
                    } else {
                        match StatisticsRepository::get_by_date(conn, &date_str).await? {
                            Some(s) => s,
                            None => DailyStatistic::zero(&date_str, chrono::Utc::now().timestamp_millis()),
                        }
                    };
                    
                    let day_name = date.format("%a").to_string();
                    points.push(ChartDataPointDTO {
                        label: day_name,
                        completion_percentage: stats.completion_percentage,
                        completed: stats.completed_count,
                        target: stats.expected_sessions,
                        snoozed: stats.snoozed_count,
                        ignored: stats.timed_out_count + stats.ignored_count,
                    });
                }
            }
            DashboardRange::Monthly => {
                let today = chrono::Utc::now().date_naive();
                let mut groups: Vec<Vec<DailyStatistic>> = vec![Vec::new(); 10];
                
                for d in 1..=31 {
                    let date_opt = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), d);
                    if let Some(date) = date_opt {
                        let date_str = date.format("%Y-%m-%d").to_string();
                        let stats = if date == today {
                            today_stats.clone()
                        } else if date < today {
                            match StatisticsRepository::get_by_date(conn, &date_str).await? {
                                Some(s) => s,
                                None => DailyStatistic::zero(&date_str, chrono::Utc::now().timestamp_millis()),
                            }
                        } else {
                            DailyStatistic::zero(&date_str, chrono::Utc::now().timestamp_millis())
                        };
                        
                        let group_idx = ((d - 1) / 3) as usize;
                        let clamped_idx = std::cmp::min(group_idx, 9);
                        groups[clamped_idx].push(stats);
                    }
                }
                
                for (idx, group_stats) in groups.iter().enumerate() {
                    if group_stats.is_empty() { continue; }
                    let start_day = idx * 3 + 1;
                    let end_day = if idx == 9 {
                        get_last_day_of_month(today.year(), today.month())
                    } else {
                        (idx * 3 + 3) as u32
                    };
                    let label = format!("Days {}-{}", start_day, end_day);
                    
                    let mut total_completed = 0;
                    let mut total_target = 0;
                    let mut total_snoozed = 0;
                    let mut total_ignored = 0;
                    let mut count = 0;
                    let mut sum_completion_pct = 0.0;
                    
                    for s in group_stats {
                        total_completed += s.completed_count;
                        total_target += s.expected_sessions;
                        total_snoozed += s.snoozed_count;
                        total_ignored += s.timed_out_count + s.ignored_count;
                        sum_completion_pct += s.completion_percentage;
                        count += 1;
                    }
                    
                    let avg_completion_pct = if count > 0 { sum_completion_pct / count as f64 } else { 0.0 };
                    
                    points.push(ChartDataPointDTO {
                        label,
                        completion_percentage: avg_completion_pct,
                        completed: total_completed,
                        target: total_target,
                        snoozed: total_snoozed,
                        ignored: total_ignored,
                    });
                }
            }
        }
        Ok(points)
    }

    async fn build_heatmap(
        conn: &mut sqlx::SqliteConnection,
        today_stats: &DailyStatistic,
    ) -> AppResult<Vec<HeatmapDayDTO>> {
        let today = chrono::Utc::now().date_naive();
        let start_date = today - chrono::Duration::days(180);
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = today.format("%Y-%m-%d").to_string();
        
        let db_stats = StatisticsRepository::get_range(conn, &start_date_str, &end_date_str).await?;
        
        let mut stats_map = std::collections::HashMap::new();
        for s in db_stats {
            stats_map.insert(s.date.clone(), s.completion_percentage);
        }
        stats_map.insert(end_date_str.clone(), today_stats.completion_percentage);
        
        let mut heatmap = Vec::new();
        for i in (0..180).rev() {
            let date = today - chrono::Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();
            let pct = stats_map.get(&date_str).copied().unwrap_or(0.0);
            heatmap.push(HeatmapDayDTO {
                date: date_str,
                completion_percentage: pct,
            });
        }
        
        Ok(heatmap)
    }

    fn build_streak(
        all_stats: &[DailyStatistic],
        today_stats: &DailyStatistic,
        active_ms_today: u64,
    ) -> AppResult<StreakDataDTO> {
        let mut current_streak = 0;
        let mut longest_streak = 0;
        
        let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
        
        for stats in all_stats {
            if stats.date == today_str {
                continue;
            }
            
            if stats.working_time_ms == 0 {
                continue;
            }
            
            if stats.completion_percentage >= 80.0 {
                current_streak += 1;
                if current_streak > longest_streak {
                    longest_streak = current_streak;
                }
            } else {
                current_streak = 0;
            }
        }
        
        let _ = today_stats;
        let _ = active_ms_today;
        
        Ok(StreakDataDTO {
            current_streak,
            longest_streak,
        })
    }

    async fn compute_today_stats(
        conn: &mut sqlx::SqliteConnection,
        today_str: &str,
        active_ms_today: u64,
        target_mode: &str,
        manual_target: i64,
        reminder_interval: i64,
    ) -> AppResult<DailyStatistic> {
        let sessions = SessionRepository::get_by_date(conn, today_str).await?;
        
        let mut completed_count = 0;
        let mut timed_out_count = 0;
        let mut ignored_count = 0;
        let mut snoozed_count = 0;
        let mut total_sessions = 0;
        
        for s in sessions {
            use std::str::FromStr;
            let status = SessionStatus::from_str(&s.status)?;
            if status == SessionStatus::Completed {
                completed_count += 1;
                total_sessions += 1;
            } else if status == SessionStatus::TimedOut {
                timed_out_count += 1;
                total_sessions += 1;
            } else if status == SessionStatus::Ignored {
                ignored_count += 1;
                total_sessions += 1;
            }
            if s.snooze_count > 0 {
                snoozed_count += 1;
            }
        }
        
        let expected_sessions = if target_mode == "automatic" {
            let active_mins = (active_ms_today / 60000) as i64;
            if active_mins > 0 {
                std::cmp::max(1, active_mins / reminder_interval)
            } else {
                0
            }
        } else {
            manual_target
        };
        
        let completion_percentage = if expected_sessions > 0 {
            (completed_count as f64 / expected_sessions as f64) * 100.0
        } else {
            0.0
        };
        
        let goal_met = if expected_sessions > 0 && completed_count >= expected_sessions {
            1
        } else {
            0
        };
        
        let now_ms = chrono::Utc::now().timestamp_millis();
        
        Ok(DailyStatistic {
            id: 0,
            date: today_str.to_string(),
            total_sessions,
            completed_count,
            snoozed_count,
            timed_out_count,
            ignored_count,
            completion_percentage,
            expected_sessions,
            goal_met,
            streak_day: 0,
            working_time_ms: active_ms_today as i64,
            created_at: now_ms,
            updated_at: now_ms,
        })
    }
}

fn get_last_day_of_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
