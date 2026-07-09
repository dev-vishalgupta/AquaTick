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
