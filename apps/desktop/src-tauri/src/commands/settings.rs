//! IPC command handlers for managing application settings.

use tauri::State;

use crate::errors::CommandError;
use crate::models::AppSettings;
use crate::services::{AutostartService, SettingsService};
use crate::state::AppState;
use crate::constants::setting_keys;

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
/// the managed in-memory cache to keep it in sync and emits an event if changed.
#[tauri::command]
pub async fn update_setting(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), CommandError> {
    use tauri::Emitter;

    // 1. Validate and save setting to the database, checking if it changed
    let changed = SettingsService::update(&state.db, &key, &value)
        .await
        .map_err(CommandError::from)?;

    if changed {
        // 2. Refresh the AppState settings cache in memory
        let updated_settings = SettingsService::load_all(&state.db)
            .await
            .map_err(CommandError::from)?;

        if let Ok(mut cache) = state.settings.lock() {
            *cache = updated_settings.clone();
        }

        // 3. Emit settings:changed event
        app_handle
            .emit(
                "settings:changed",
                SettingsChangedPayload {
                    settings: updated_settings,
                },
            )
            .ok();

        // 4. Handle Phase 7B side-effects: OS Autostart
        if key == setting_keys::START_ON_BOOT {
            let enabled = value == "true";
            if let Err(e) = AutostartService::sync(&app_handle, enabled) {
                log::error!("AutostartService: sync failed: {}", e);
                // Non-fatal — setting is saved, OS sync failed. Log and continue.
            }
        }
    }

    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct SettingsChangedPayload {
    settings: AppSettings,
}
