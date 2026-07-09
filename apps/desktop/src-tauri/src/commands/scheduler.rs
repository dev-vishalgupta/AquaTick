//! IPC command handlers for controlling the Reminder Scheduler.

use tauri::{AppHandle, State};

use crate::errors::CommandError;
use crate::scheduler::{SchedulerService, SchedulerState};
use crate::state::AppState;

/// Starts the reminder scheduler using the configured interval from settings.
#[tauri::command]
pub async fn start_scheduler(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let interval_ms = {
        let s = state.settings.lock().map_err(|_| {
            CommandError {
                code: "INTERNAL_ERROR".to_string(),
                message: "Failed to read settings".to_string(),
            }
        })?;
        (s.reminder_interval_minutes as u64) * 60 * 1_000
    };

    SchedulerService::start(app_handle, &state, interval_ms)
        .await
        .map_err(CommandError::from)
}

/// Stops the reminder scheduler completely.
#[tauri::command]
pub fn stop_scheduler(state: State<'_, AppState>) -> Result<(), CommandError> {
    SchedulerService::stop(&state);
    Ok(())
}

/// Returns the current scheduler state (stopped, running, paused, triggered).
#[tauri::command]
pub fn get_scheduler_status(state: State<'_, AppState>) -> Result<SchedulerState, CommandError> {
    Ok(SchedulerService::status(&state))
}
