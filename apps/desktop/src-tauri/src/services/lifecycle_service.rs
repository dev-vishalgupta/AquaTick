//! LifecycleService — Orchestrated Application Shutdown.
//!
//! Controls the clean teardown and shutdown sequence of all backend services
//! (SchedulerService, ActivityMonitorService) prior to exit.

use tauri::Manager;
use crate::state::AppState;

pub struct LifecycleService;

impl LifecycleService {
    /// Coordinates an ordered shutdown of all background services
    /// and terminates the application process.
    ///
    /// # Ordering (invariant LS-1)
    /// 1. Stop SchedulerService — prevents new sessions from being created.
    /// 2. Stop ActivityMonitorService — prevents spurious pause/resume after shutdown.
    /// 3. Exit process.
    ///
    /// # Caller constraint (invariant LS-2)
    /// Only tray.rs is permitted to call this function.
    pub async fn shutdown(app_handle: tauri::AppHandle) {
        if let Some(state) = app_handle.try_state::<AppState>() {
            state.scheduler.stop().await;
            state.activity_monitor.stop().await;
        }
        app_handle.exit(0);
    }
}
