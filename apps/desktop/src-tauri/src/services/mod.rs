//! Application services layer.
//!
//! Exposes domain logic (validation, aggregation, transition enforcement)
//! for settings, sessions, statistics, internal scheduling, active usage
//! monitoring, and the reminder engine.

pub mod activity_monitor_service;
pub mod reminder_engine_service;
pub mod scheduler_service;
pub mod session_service;
pub mod settings_service;
pub mod statistics_service;

pub use activity_monitor_service::{ActivityMonitorService, MonitorState};
pub use reminder_engine_service::{
    build_reminder_due_callback, EngineState, ReminderEngineService,
};
pub use scheduler_service::{OnReminderDue, SchedulerService, SchedulerState};
pub use session_service::SessionService;
pub use settings_service::SettingsService;
pub use statistics_service::StatisticsService;

