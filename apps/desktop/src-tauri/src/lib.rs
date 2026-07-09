//! AquaTick — Tauri application entry point.
//!
//! `run()` is called from `main.rs`. It wires all backend modules together:
//! - Registers Tauri plugins (sql, window-state).
//! - Runs the database initializer in an async setup task.
//! - Constructs `AppState` and registers it with Tauri's state manager.
//!
//! No business logic lives here. All logic is delegated to the module tree.

#![allow(clippy::module_inception)]

mod commands;
mod constants;
mod database;
mod errors;
mod models;
mod repositories;
mod services;
mod state;

use tauri::{Emitter, Manager};

#[derive(Debug, Clone, serde::Serialize)]
struct AppReadyPayload {
    settings: models::AppSettings,
    today_stats: models::DailyStatistic,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            // Resolve the platform-specific app data directory before spawning.
            // On Windows: %APPDATA%\com.aquatick.app
            let app_data_dir = app.path().app_data_dir()?;
            let app_handle = app.handle().clone();

            // Spawn the async initialization sequence.
            tauri::async_runtime::spawn(async move {
                match database::init_db(&app_data_dir).await {
                    Ok(pool) => {
                        let now_ms = chrono::Utc::now().timestamp_millis();

                        // 1. Recover stale sessions left in 'triggered' state across crash/shutdown
                        let recovered = services::SessionService::recover_stale(
                            &pool,
                            now_ms,
                            constants::DEFAULT_SESSION_TIMEOUT_MINUTES,
                        )
                        .await
                        .unwrap_or(0);

                        if recovered > 0 {
                            log::info!("Recovered {} stale triggered session(s) on startup.", recovered);
                        }

                        // 2. Load all settings from database to construct AppState settings cache
                        let settings = services::SettingsService::load_all(&pool)
                            .await
                            .unwrap_or_default();

                        // 3. Pre-fetch today's statistics
                        let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let today_stats = services::StatisticsService::get_daily_statistics(&pool, &today_str)
                            .await
                            .unwrap_or_else(|_| models::DailyStatistic::zero(&today_str, now_ms));

                        // 4. Initialize and manage shared AppState
                        app_handle.manage(state::AppState::new(pool, settings.clone()));

                        // 5. Emit app ready event to notify React frontend with initial payload
                        let payload = AppReadyPayload {
                            settings,
                            today_stats,
                        };
                        app_handle.emit("aquatick://app/ready", payload).ok();

                        log::info!("AquaTick backend initialized successfully. Emitted app/ready.");
                    }
                    Err(e) => {
                        // A database failure at startup is fatal.
                        log::error!("FATAL: Database initialization failed: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::statistics::get_daily_statistics,
            commands::statistics::get_weekly_statistics,
            commands::session::get_session_by_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

