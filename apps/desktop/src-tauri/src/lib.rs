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

use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, serde::Serialize)]
struct AppReadyPayload {
    settings:    models::AppSettings,
    today_stats: models::DailyStatistic,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let app_handle   = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match database::init_db(&app_data_dir).await {
                    Ok(pool) => {
                        // 1. Load settings first (needed by engine callback).
                        let settings = services::SettingsService::load_all(&pool)
                            .await
                            .unwrap_or_default();

                        // 2. Pre-fetch today's statistics.
                        let now_ms    = chrono::Utc::now().timestamp_millis();
                        let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let today_stats = services::StatisticsService::get_daily_statistics(
                            &pool, &today_str,
                        )
                        .await
                        .unwrap_or_else(|_| {
                            models::DailyStatistic::zero(&today_str, now_ms)
                        });

                        // 3. Initialize AppState and extract clones before manage().
                        let app_state    = state::AppState::new(pool.clone(), settings.clone());
                        let scheduler_h  = app_state.scheduler.clone();
                        let monitor_h    = app_state.activity_monitor.clone();
                        let engine_h     = app_state.reminder_engine.clone();
                        app_state.reminder_engine.set_app_handle(app_handle.clone());
                        app_handle.manage(app_state);

                        // 4. Recover stale sessions from a previous crash/shutdown.
                        engine_h.recover_stale_sessions(&pool).await;

                        // 5. Build the on_reminder_due callback that wires
                        //    SchedulerService → ReminderEngineService.
                        let callback = services::build_reminder_due_callback(
                            engine_h.clone(),
                            pool.clone(),
                            settings.clone(),
                            scheduler_h.clone(),
                        );

                        // 6. Start the reminder scheduler with the initial interval.
                        let interval = Duration::from_secs(
                            (settings.reminder_interval_minutes as u64).saturating_mul(60)
                        );
                        if settings.reminder_enabled {
                            scheduler_h.start(interval, callback).await;
                            log::info!(
                                "Scheduler: Started (interval: {} min).",
                                settings.reminder_interval_minutes
                            );
                        } else {
                            log::info!("Scheduler: Reminders disabled — scheduler not started.");
                        }

                        // 7. Start the Active Usage Monitor.
                        //    It pauses / resumes `scheduler_h` based on idle and sleep events.
                        monitor_h.start(scheduler_h).await;
                        log::info!("ActivityMonitor: Running.");

                        // 8. Emit app/ready to notify the React frontend.
                        let payload = AppReadyPayload { settings, today_stats };
                        app_handle.emit("aquatick://app/ready", payload).ok();

                        log::info!("AquaTick backend initialized successfully. Emitted app/ready.");
                    }
                    Err(e) => {
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
            commands::session::complete_session,
            commands::session::snooze_session,
            commands::session::timeout_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

