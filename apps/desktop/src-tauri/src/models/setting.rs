//! Data models for the `settings` table and the deserialized `AppSettings` struct.
//!
//! The `settings` table stores all preferences as typed key-value rows.
//! `SettingRow` mirrors the raw DB row; `AppSettings` is the deserialized
//! working copy held in `AppState` and returned over IPC.

use serde::{Deserialize, Serialize};

// ── Raw DB row ────────────────────────────────────────────────────────────────

/// Mirrors a single row in the `settings` table.
///
/// `value` is always a string in the database. Cast to the correct Rust type
/// using `value_type` with `SettingValueType::from_str`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    /// One of: `string | integer | real | boolean | json`
    pub value_type: String,
    /// Unix milliseconds — when this row was last written.
    pub updated_at: i64,
}

// ── Value type enum ───────────────────────────────────────────────────────────

/// Valid cast targets for a `SettingRow.value` string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingValueType {
    String,
    Integer,
    Real,
    Boolean,
    Json,
}

impl SettingValueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String  => "string",
            Self::Integer => "integer",
            Self::Real    => "real",
            Self::Boolean => "boolean",
            Self::Json    => "json",
        }
    }
}

impl std::str::FromStr for SettingValueType {
    type Err = crate::errors::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "string"  => Ok(Self::String),
            "integer" => Ok(Self::Integer),
            "real"    => Ok(Self::Real),
            "boolean" => Ok(Self::Boolean),
            "json"    => Ok(Self::Json),
            other => Err(crate::errors::AppError::Validation(format!(
                "Unknown value_type '{}'. Expected: string | integer | real | boolean | json",
                other
            ))),
        }
    }
}

// ── Deserialized settings ─────────────────────────────────────────────────────

/// Strongly-typed application settings.
///
/// Populated from `SettingRow` rows by `SettingsService::load_all`.
/// Held in `AppState.settings` (behind a `Mutex`) as the live cache.
/// Returned as the IPC payload for `get_settings`.
///
/// Adding a new setting requires:
///   1. A new field here with its default.
///   2. A new key in `constants::setting_keys`.
///   3. A seed row in `database::migrations::seed_defaults`.
///   4. Deserialization logic in `services::settings_service::load_all`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub schema_version: i64,
    pub app_version: String,
    pub reminder_interval_minutes: i64,
    pub reminder_enabled: bool,
    pub sound_enabled: bool,
    /// 0.0 – 1.0 (displayed to user as percentage)
    pub sound_volume: f64,
    pub character_id: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        use crate::constants::*;
        Self {
            schema_version: SCHEMA_VERSION,
            app_version: APP_VERSION.to_string(),
            reminder_interval_minutes: DEFAULT_REMINDER_INTERVAL_MINUTES,
            reminder_enabled: DEFAULT_REMINDER_ENABLED,
            sound_enabled: DEFAULT_SOUND_ENABLED,
            sound_volume: DEFAULT_SOUND_VOLUME,
            character_id: DEFAULT_CHARACTER_ID.to_string(),
        }
    }
}
