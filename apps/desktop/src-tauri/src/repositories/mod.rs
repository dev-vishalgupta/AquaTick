//! Data access layer (Repository Pattern).
//!
//! Exposes pure database operations (CRUD) for settings, sessions, and daily statistics.

pub mod settings_repo;
pub mod session_repo;
pub mod statistics_repo;

pub use settings_repo::SettingsRepository;
pub use session_repo::SessionRepository;
pub use statistics_repo::StatisticsRepository;
