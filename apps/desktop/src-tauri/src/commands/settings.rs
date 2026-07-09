//! IPC command handlers for managing application settings.

use tauri::State;

use crate::errors::CommandError;
use crate::models::AppSettings;
use crate::services::SettingsService;
use crate::state::AppState;

/// Retrieves the current application settings.
///
/// Reads from the database to ensure the most up-to-date values.
#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, CommandError> {
    let settings = SettingsService::load_all(&state.db)
        .await
        .map_err(CommandError::from)?;

    Ok(settings)
}

/// Updates a specific setting key-value pair.
///
/// Validates the input format and updates the database, then refreshes
/// the managed in-memory cache to keep it in sync.
#[tauri::command]
pub async fn update_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), CommandError> {
    // 1. Validate and save setting to the database
    SettingsService::update(&state.db, &key, &value)
        .await
        .map_err(CommandError::from)?;

    // 2. Refresh the AppState settings cache in memory
    let updated_settings = SettingsService::load_all(&state.db)
        .await
        .map_err(CommandError::from)?;

    if let Ok(mut cache) = state.settings.lock() {
        *cache = updated_settings;
    }

    Ok(())
}
