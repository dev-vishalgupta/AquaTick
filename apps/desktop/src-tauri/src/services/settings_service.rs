//! Service layer for application settings.
//!
//! Orchestrates the `SettingsRepository` and handles type casting, validation, and defaults.

use sqlx::{Pool, Sqlite};
use std::str::FromStr;

use crate::constants::{setting_keys, VALID_SETTING_KEYS};
use crate::errors::{AppError, AppResult};
use crate::models::{AppSettings, SettingRow, SettingValueType};
use crate::repositories::SettingsRepository;

/// Service for managing application settings.
pub struct SettingsService;

impl SettingsService {
    /// Loads all settings from the database and deserializes them into the strongly-typed `AppSettings` struct.
    pub async fn load_all(pool: &Pool<Sqlite>) -> AppResult<AppSettings> {
        let rows = SettingsRepository::get_all(pool).await?;
        let mut settings = AppSettings::default();

        for row in rows {
            match row.key.as_str() {
                setting_keys::DB_SCHEMA_VERSION => {
                    if let Ok(v) = row.value.parse::<i64>() {
                        settings.schema_version = v;
                    }
                }
                setting_keys::APP_VERSION => {
                    settings.app_version = row.value;
                }
                setting_keys::REMINDER_INTERVAL_MINUTES => {
                    if let Ok(v) = row.value.parse::<i64>() {
                        settings.reminder_interval_minutes = v;
                    }
                }
                setting_keys::REMINDER_ENABLED => {
                    if let Ok(v) = row.value.parse::<bool>() {
                        settings.reminder_enabled = v;
                    }
                }
                setting_keys::REMINDER_SOUND_ENABLED => {
                    if let Ok(v) = row.value.parse::<bool>() {
                        settings.sound_enabled = v;
                    }
                }
                setting_keys::REMINDER_SOUND_VOLUME => {
                    if let Ok(v) = row.value.parse::<f64>() {
                        settings.sound_volume = v;
                    }
                }
                setting_keys::CHARACTER_ID => {
                    settings.character_id = row.value;
                }
                _ => {} // Ignore unrecognized keys in database
            }
        }

        Ok(settings)
    }

    /// Fetches a single setting and parses it into the requested type.
    pub async fn get_typed<T>(pool: &Pool<Sqlite>, key: &str) -> AppResult<T>
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let row = SettingsRepository::get_by_key(pool, key).await?
            .ok_or_else(|| AppError::NotFound(format!("Setting key '{}' not found", key)))?;

        row.value.parse::<T>().map_err(|e| {
            AppError::Serialization(format!(
                "Failed to parse setting '{}' with value '{}': {}",
                key, row.value, e
            ))
        })
    }

    /// Validates and updates a setting value.
    pub async fn update(pool: &Pool<Sqlite>, key: &str, value: &str) -> AppResult<()> {
        // 1. Validate that the key is well-known
        if !VALID_SETTING_KEYS.contains(&key) {
            return Err(AppError::Validation(format!("Invalid setting key: '{}'", key)));
        }

        // 2. Fetch the metadata (value_type) from DB if exists, or determine it from the schema defaults.
        let existing = SettingsRepository::get_by_key(pool, key).await?;
        let value_type = match existing {
            Some(row) => row.value_type,
            None => {
                // If key is valid but not in DB yet (unlikely due to seeding, but possible),
                // map it to its expected value type.
                Self::determine_value_type(key)?.as_str().to_string()
            }
        };

        // 3. Validate value syntax matches the setting value type rules
        Self::validate_setting_value(key, value, &value_type)?;

        // 4. Save to repository
        let now_ms = chrono::Utc::now().timestamp_millis();
        SettingsRepository::upsert(pool, key, value, &value_type, now_ms).await?;

        Ok(())
    }

    // ── Helper functions ────────────────────────────────────────────────────────

    /// Resolves the default `SettingValueType` for a well-known key.
    fn determine_value_type(key: &str) -> AppResult<SettingValueType> {
        match key {
            setting_keys::DB_SCHEMA_VERSION | setting_keys::REMINDER_INTERVAL_MINUTES => {
                Ok(SettingValueType::Integer)
            }
            setting_keys::REMINDER_ENABLED | setting_keys::REMINDER_SOUND_ENABLED => {
                Ok(SettingValueType::Boolean)
            }
            setting_keys::REMINDER_SOUND_VOLUME => Ok(SettingValueType::Real),
            setting_keys::APP_VERSION | setting_keys::CHARACTER_ID => Ok(SettingValueType::String),
            _ => Err(AppError::Validation(format!("Unsupported setting key: '{}'", key))),
        }
    }

    /// Validates the format and bounds of a setting value.
    fn validate_setting_value(key: &str, value: &str, value_type_str: &str) -> AppResult<()> {
        let value_type = SettingValueType::from_str(value_type_str)?;

        match value_type {
            SettingValueType::Integer => {
                let parsed = value.parse::<i64>().map_err(|_| {
                    AppError::Validation(format!("Setting '{}' must be an integer", key))
                })?;

                // Rule validation
                if key == setting_keys::REMINDER_INTERVAL_MINUTES && parsed <= 0 {
                    return Err(AppError::Validation(
                        "reminder.interval_minutes must be greater than zero".to_string(),
                    ));
                }
            }
            SettingValueType::Real => {
                let parsed = value.parse::<f64>().map_err(|_| {
                    AppError::Validation(format!("Setting '{}' must be a real number", key))
                })?;

                // Rule validation
                if key == setting_keys::REMINDER_SOUND_VOLUME && !(0.0..=1.0).contains(&parsed) {
                    return Err(AppError::Validation(
                        "reminder.sound_volume must be between 0.0 and 1.0".to_string(),
                    ));
                }
            }
            SettingValueType::Boolean => {
                if value != "true" && value != "false" {
                    return Err(AppError::Validation(format!(
                        "Setting '{}' must be a boolean ('true' or 'false')",
                        key
                    )));
                }
            }
            SettingValueType::String => {
                if value.trim().is_empty() {
                    return Err(AppError::Validation(format!(
                        "Setting '{}' value cannot be empty",
                        key
                    )));
                }
            }
            SettingValueType::Json => {
                serde_json::from_str::<serde_json::Value>(value).map_err(|_| {
                    AppError::Validation(format!("Setting '{}' must be valid JSON", key))
                })?;
            }
        }

        Ok(())
    }
}
