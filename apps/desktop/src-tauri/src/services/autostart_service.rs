use tauri_plugin_autostart::ManagerExt;
use crate::errors::{AppError, AppResult};

/// Sole owner of OS autostart registration.
pub struct AutostartService;

impl AutostartService {
    /// Enables or disables OS autostart based on `enabled`.
    /// No-op if the OS state already matches.
    pub fn sync(app_handle: &tauri::AppHandle, enabled: bool) -> AppResult<()> {
        let autolaunch = app_handle.autolaunch();

        // In debug builds (development mode), OS autostart registration must be disabled
        // to prevent target/debug/aquatick.exe (which expects Vite devUrl http://localhost:1420)
        // from being registered in the Windows startup registry.
        if cfg!(debug_assertions) {
            log::info!("AutostartService: Skipping OS autostart registration in debug mode.");
            if autolaunch.is_enabled().unwrap_or(false) {
                let _ = autolaunch.disable();
            }
            return Ok(());
        }
        
        let is_enabled = autolaunch.is_enabled().unwrap_or(false);
        if is_enabled == enabled {
            return Ok(());
        }

        if enabled {
            autolaunch.enable().map_err(|e| {
                AppError::Internal(format!("Failed to enable autostart: {}", e))
            })?;
        } else {
            autolaunch.disable().map_err(|e| {
                AppError::Internal(format!("Failed to disable autostart: {}", e))
            })?;
        }

        Ok(())
    }

    /// Reconciles OS autostart with the stored setting at startup.
    /// Called once from lib.rs after settings are loaded.
    pub fn sync_on_startup(app_handle: &tauri::AppHandle, enabled: bool) -> AppResult<()> {
        let autolaunch = app_handle.autolaunch();
        
        let is_enabled = autolaunch.is_enabled().unwrap_or(false);
        if is_enabled != enabled {
            log::info!(
                "AutostartService: Reconciling OS state (OS: {}, Setting: {})",
                is_enabled,
                enabled
            );
            Self::sync(app_handle, enabled)?;
        }
        
        Ok(())
    }
}
