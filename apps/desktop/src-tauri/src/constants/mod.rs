//! Application-wide constants.
//!
//! All `DEFAULT_*` values match the Architecture Design Document §11.
//! All `setting_keys::*` constants match the `settings` table schema.
//!
//! No logic lives here — only named literals.

// ── Reminder & Scheduling ─────────────────────────────────────────────────────

/// Interval between Hydration Sessions (minutes).
pub const DEFAULT_REMINDER_INTERVAL_MINUTES: i64 = 60;

/// Duration of a single snooze (minutes).
pub const DEFAULT_SNOOZE_DURATION_MINUTES: i64 = 5;

/// OS idle duration before reminder scheduling pauses (minutes).
pub const DEFAULT_IDLE_TIMEOUT_MINUTES: i64 = 30;

/// Duration before a triggered session auto-resolves as `timed_out` (minutes).
pub const DEFAULT_SESSION_TIMEOUT_MINUTES: i64 = 2;

/// Whether reminders are enabled on first launch.
pub const DEFAULT_REMINDER_ENABLED: bool = true;

// ── Audio ─────────────────────────────────────────────────────────────────────

/// Default sound volume (0.0 – 1.0, displayed as 80%).
pub const DEFAULT_SOUND_VOLUME: f64 = 0.8;

/// Whether sound is enabled on first launch.
pub const DEFAULT_SOUND_ENABLED: bool = true;

// ── Character ─────────────────────────────────────────────────────────────────

/// Default character manifest ID.
pub const DEFAULT_CHARACTER_ID: &str = "female_default";

// ── Window ────────────────────────────────────────────────────────────────────

/// Initial window width on first launch (px).
pub const DEFAULT_WINDOW_WIDTH: u32 = 1000;

/// Initial window height on first launch (px).
pub const DEFAULT_WINDOW_HEIGHT: u32 = 750;

/// Minimum resizable window width (px).
pub const DEFAULT_MIN_WINDOW_WIDTH: u32 = 800;

/// Minimum resizable window height (px).
pub const DEFAULT_MIN_WINDOW_HEIGHT: u32 = 600;

// ── Application ───────────────────────────────────────────────────────────────

/// SQLite filename stored in the Tauri app data directory.
pub const DEFAULT_DATABASE_NAME: &str = "aquatick.sqlite";

/// Whether to register with OS autostart on first launch.
pub const DEFAULT_START_ON_BOOT: bool = true;

/// Semantic version of the application.
pub const APP_VERSION: &str = "0.1.0";

// ── Database ──────────────────────────────────────────────────────────────────

/// Current schema version. Increment when adding a new migration.
pub const SCHEMA_VERSION: i64 = 1;

// ── Active Usage ──────────────────────────────────────────────────────────────

/// How often Rust polls the OS idle API (seconds).
pub const IDLE_POLL_INTERVAL_SECONDS: u64 = 10;

/// If the actual gap between two consecutive polls exceeds the poll interval
/// multiplied by this factor, the system is assumed to have slept and woken.
///
/// Example: poll interval = 10 s, multiplier = 4 → gap > 40 s → sleep detected.
pub const SLEEP_GAP_MULTIPLIER: u32 = 4;

// ── Well-known Setting Keys ───────────────────────────────────────────────────

/// String keys used to read/write rows in the `settings` table.
/// Adding a new setting requires: a new key here, a default value in
/// `database::migrations::seed_defaults`, and a field in `models::AppSettings`.
pub mod setting_keys {
    pub const DB_SCHEMA_VERSION: &str = "db.schema_version";
    pub const APP_VERSION: &str = "app.version";
    pub const REMINDER_INTERVAL_MINUTES: &str = "reminder.interval_minutes";
    pub const REMINDER_ENABLED: &str = "reminder.enabled";
    pub const REMINDER_SOUND_ENABLED: &str = "reminder.sound_enabled";
    pub const REMINDER_SOUND_VOLUME: &str = "reminder.sound_volume";
    pub const CHARACTER_ID: &str = "character.id";
}

/// All valid setting keys. Used by SettingsService for validation.
/// Invariant: every key in `setting_keys` must appear here.
pub const VALID_SETTING_KEYS: &[&str] = &[
    setting_keys::DB_SCHEMA_VERSION,
    setting_keys::APP_VERSION,
    setting_keys::REMINDER_INTERVAL_MINUTES,
    setting_keys::REMINDER_ENABLED,
    setting_keys::REMINDER_SOUND_ENABLED,
    setting_keys::REMINDER_SOUND_VOLUME,
    setting_keys::CHARACTER_ID,
];
