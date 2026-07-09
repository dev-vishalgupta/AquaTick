//! IPC command handlers for retrieving session statistics.

use tauri::State;

use crate::errors::CommandError;
use crate::models::{DailyStatistic, WeeklyStatistic};
use crate::services::StatisticsService;
use crate::state::AppState;

/// Retrieves the daily statistics for a calendar date (ISO YYYY-MM-DD).
#[tauri::command]
pub async fn get_daily_statistics(
    state: State<'_, AppState>,
    date: String,
) -> Result<DailyStatistic, CommandError> {
    let stats = StatisticsService::get_daily_statistics(&state.db, &date)
        .await
        .map_err(CommandError::from)?;

    Ok(stats)
}

/// Retrieves the aggregated weekly statistics starting on a specific Monday (ISO YYYY-MM-DD).
#[tauri::command]
pub async fn get_weekly_statistics(
    state: State<'_, AppState>,
    week_start: String,
) -> Result<WeeklyStatistic, CommandError> {
    let stats = StatisticsService::get_weekly_statistics(&state.db, &week_start)
        .await
        .map_err(CommandError::from)?;

    Ok(stats)
}
