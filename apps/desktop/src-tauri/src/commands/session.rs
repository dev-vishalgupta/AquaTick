//! IPC command handlers for retrieving session records.

use tauri::State;

use crate::errors::CommandError;
use crate::models::HydrationSession;
use crate::services::SessionService;
use crate::state::AppState;

/// Retrieves details of a single hydration session by ID.
#[tauri::command]
pub async fn get_session_by_id(
    state: State<'_, AppState>,
    id: i64,
) -> Result<HydrationSession, CommandError> {
    let session = SessionService::get_by_id(&state.db, id)
        .await
        .map_err(CommandError::from)?;

    Ok(session)
}

/// Marks a hydration session as completed.
#[tauri::command]
pub async fn complete_session(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), CommandError> {
    let active_ms_today = {
        let guard = state.active_ms_today.lock().map_err(|e| CommandError {
            code: "MUTEX_ERROR".to_string(),
            message: e.to_string(),
        })?;
        *guard as i64
    };

    state.reminder_engine
        .complete_session(&state.db, id, active_ms_today, &state.scheduler)
        .await
        .map_err(CommandError::from)?;

    Ok(())
}

/// Snoozes an active hydration session.
#[tauri::command]
pub async fn snooze_session(
    state: State<'_, AppState>,
    id: i64,
    delay_minutes: i64,
) -> Result<(), CommandError> {
    state.reminder_engine
        .snooze_session(&state.db, id, delay_minutes, &state.scheduler)
        .await
        .map_err(CommandError::from)?;

    Ok(())
}

/// Times out an active hydration session.
#[tauri::command]
pub async fn timeout_session(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), CommandError> {
    let active_ms_today = {
        let guard = state.active_ms_today.lock().map_err(|e| CommandError {
            code: "MUTEX_ERROR".to_string(),
            message: e.to_string(),
        })?;
        *guard as i64
    };

    state.reminder_engine
        .timeout_session(&state.db, id, active_ms_today, &state.scheduler)
        .await
        .map_err(CommandError::from)?;

    Ok(())
}

/// Explicitly ignores an active hydration session.
#[tauri::command]
pub async fn ignore_session(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), CommandError> {
    let active_ms_today = {
        let guard = state.active_ms_today.lock().map_err(|e| CommandError {
            code: "MUTEX_ERROR".to_string(),
            message: e.to_string(),
        })?;
        *guard as i64
    };

    state.reminder_engine
        .ignore_session(&state.db, id, active_ms_today, &state.scheduler)
        .await
        .map_err(CommandError::from)?;

    Ok(())
}

