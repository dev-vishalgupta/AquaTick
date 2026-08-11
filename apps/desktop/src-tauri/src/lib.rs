//! AquaTick — Tauri application entry point.
//!
//! `run()` is called from `main.rs`. It wires all backend modules together:
//! - Registers Tauri plugins (sql, window-state).
//! - Runs the database initializer in an async setup task.
//! - Constructs `AppState` and registers it with Tauri's state manager.
//!
//! No business logic lives here. All logic is delegated to the module tree.
//!
//! # Startup Modes
//!
//! | Mode          | Trigger                  | Main Window |
//! |---------------|--------------------------|-------------|
//! | Normal Launch | User opens AquaTick      | Visible     |
//! | Boot Launch   | `--boot` CLI flag        | Hidden      |
//!
//! Boot Launch is used when Windows launches AquaTick at login via the autostart
//! registry entry registered by `AutostartService`. The `--boot` flag is appended
//! to the registry command by `tauri-plugin-autostart` during Phase 7B.

#![allow(clippy::module_inception)]

mod commands;
mod constants;
mod database;
mod errors;
mod models;
mod repositories;
mod services;
mod state;
mod tray;

use std::time::Duration;
use tauri::{Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Clone, serde::Serialize)]
struct AppReadyPayload {
    settings:    models::AppSettings,
    today_stats: models::DailyStatistic,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Detect Boot Launch before the Tauri builder runs.
    //
    // When Windows launches AquaTick via the autostart registry entry (Phase 7B),
    // the `--boot` flag is appended by tauri-plugin-autostart. Normal launches
    // (user double-clicks the app) do not carry this flag.
    let boot_mode = std::env::args().any(|a| a == "--boot");

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Phase 7B: tauri-plugin-autostart registration
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--boot"]),
        ))
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .setup(move |app| {
            // ── Boot Launch: hide main window immediately ──────────────────────
            //
            // Invariant BL-1: In Boot Launch the main window MUST be hidden before
            // any async work (DB init, service starts) begins. This prevents any
            // window flash during Windows login, because the window's webview has
            // not yet rendered content at this point in setup().
            if boot_mode {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                    log::info!("Startup: Boot Launch detected (--boot). Main window hidden.");
                }
            }

            // Set up system tray
            tray::setup_tray(app)?;

            // Query primary monitor dimensions
            let size = match app.primary_monitor() {
                Ok(Some(monitor)) => monitor.size().clone(),
                _ => tauri::PhysicalSize { width: 1920, height: 1080 },
            };

            // Create persistent, transparent, always-on-top overlay window
            let overlay_window = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("overlay.html".into())
            )
            .title("")
            .inner_size(size.width as f64, size.height as f64)
            .position(0.0, 0.0)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .shadow(false)
            .build()?;

            let _ = overlay_window.set_ignore_cursor_events(true);

            let app_data_dir = app.path().app_data_dir()?;
            let app_handle   = app.handle().clone();

            // Register presentation lifecycle window controls (decoupled from ReminderEngine)
            let app_handle_presented = app_handle.clone();
            app_handle.listen("reminder:presented", move |_event| {
                log::info!("Presentation manager: reminder:presented event. Showing overlay window.");
                if let Some(win) = app_handle_presented.get_webview_window("overlay") {
                    let _ = win.show();
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_focus();
                }
            });

            let app_handle_closed = app_handle.clone();
            app_handle.listen("presentation:closed", move |_event| {
                log::info!("Presentation manager: presentation:closed event. Hiding overlay window.");
                if let Some(win) = app_handle_closed.get_webview_window("overlay") {
                    let _ = win.hide();
                }
            });

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

                        // 8. Phase 7B: AutostartService::sync_on_startup
                        // Invariant AS-1: Reconciliation failures MUST be logged but MUST NOT
                        // abort startup. The application must always reach step 9 (app/ready).
                        if let Err(e) = services::AutostartService::sync_on_startup(&app_handle, settings.start_on_boot) {
                            log::error!("AutostartService: startup reconciliation failed: {}", e);
                        }

                        // 9. Emit app/ready to notify the React frontend.
                        //
                        // Invariant BR-1: app/ready is always the LAST step in startup.
                        // In Boot Launch, the main window is already hidden (step BL-1 above).
                        // The overlay window receives this event through its own EventCoordinator.
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
            commands::statistics::get_dashboard,
            commands::session::get_session_by_id,
            commands::session::complete_session,
            commands::session::snooze_session,
            commands::session::timeout_session,
            commands::session::ignore_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
