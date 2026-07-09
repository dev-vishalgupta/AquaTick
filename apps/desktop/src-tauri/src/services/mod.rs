//! Application services layer.
//!
//! Exposes domain logic (validation, aggregation, transition enforcement)
//! for settings, sessions, statistics, and internal scheduling.

pub mod scheduler_service;
pub mod session_service;
pub mod settings_service;
pub mod statistics_service;

pub use scheduler_service::{SchedulerService, SchedulerState};
pub use session_service::SessionService;
pub use settings_service::SettingsService;
pub use statistics_service::StatisticsService;
