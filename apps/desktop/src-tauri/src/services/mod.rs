//! Application services layer.
//!
//! Exposes domain logic (validation, aggregation, transition enforcement)
//! for settings, sessions, and daily statistics.

pub mod settings_service;
pub mod session_service;
pub mod statistics_service;

pub use settings_service::SettingsService;
pub use session_service::SessionService;
pub use statistics_service::StatisticsService;
