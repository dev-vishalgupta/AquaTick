//! Shared application state.
//!
//! `AppState` is registered with Tauri via `app.manage(AppState::new(...))` in
//! `lib.rs`. Command handlers receive it as `tauri::State<'_, AppState>`.
//!
//! # Field ownership rules (Architecture §4, Invariants §8)
//!
//! | Field                | Phase   | Purpose                         |
//! |----------------------|---------|---------------------------------|
//! | `db`                 | Phase 3A | Live database pool              |
//! | `settings`           | Phase 3A | Eagerly-loaded settings cache   |
//! | `active_session_id`  | Phase 3E | Session lifecycle               |
//! | `scheduler_paused`   | Phase 3E | Active Usage SM                 |
//! | `remaining_ms`       | Phase 3E | Timer preservation              |
//! | `active_ms_today`    | Phase 3E | Usage time tracking             |
//! | `reminder_timer`     | Phase 4A | Reminder Engine SM              |
//! | `snooze_timer`       | Phase 4  | Reminder Engine SM (future)     |
//! | `timeout_timer`      | Phase 4  | Reminder Engine SM (future)     |

use sqlx::{Pool, Sqlite};
use std::sync::Mutex;
use tokio::task::AbortHandle;

use crate::models::AppSettings;

/// Tauri-managed shared state for the AquaTick backend.
///
/// All mutable fields are wrapped in `Mutex` so they can be safely accessed
/// from concurrent Tauri command handlers.
///
/// # Invariant T-1 / T-2 / T-3 (State Machine §8)
/// Only one reminder timer, one snooze timer, and one timeout timer may exist
/// at any time. These invariants are enforced by the Scheduler/Reminder Engine
/// by cancelling the previous handle before creating a new one.
pub struct AppState {
    // ── Database ──────────────────────────────────────────────────────────────

    /// SQLite connection pool — the only database access point in the backend.
    ///
    /// `Pool<Sqlite>` is internally `Arc`-wrapped and `Clone`-able.
    /// Repositories and services receive a reference to this pool.
    pub db: Pool<Sqlite>,

    // ── Settings cache ────────────────────────────────────────────────────────

    /// Eagerly-loaded settings cache.
    ///
    /// Populated from the database at startup by `SettingsService::load_all`.
    /// Updated whenever a `update_setting` command succeeds.
    /// Commands read from this cache instead of making redundant DB calls.
    pub settings: Mutex<AppSettings>,

    // ── Session state ─────────────────────────────────────────────────────────

    /// ID of the currently open Hydration Session.
    ///
    /// `None` when no session is active.
    /// Set to `Some(id)` when the reminder timer fires.
    /// Cleared to `None` when a session reaches `completed` or `timed_out`.
    ///
    /// # Invariant S-1 / S-6
    /// Only one active session may exist at any time.
    pub active_session_id: Mutex<Option<i64>>,

    // ── Scheduler state ───────────────────────────────────────────────────────

    /// Whether reminder scheduling is currently paused.
    ///
    /// Set to `true` when the user goes idle, the screen locks, or the OS sleeps.
    /// Set to `false` when activity is detected again.
    ///
    /// # Invariant SC-1
    /// No Hydration Session may be triggered while `scheduler_paused == true`.
    pub scheduler_paused: Mutex<bool>,

    /// Remaining milliseconds on the paused reminder timer.
    ///
    /// Captured at the moment the scheduler pauses.
    /// Used to resume the timer from where it left off — never from the full interval.
    /// Cleared after the timer is successfully rescheduled on resume.
    ///
    /// # Invariant T-5 / T-6
    pub remaining_ms: Mutex<Option<u64>>,

    /// Cumulative active milliseconds recorded during the current calendar day.
    ///
    /// Resets at midnight. Written to `daily_statistics` when a session resolves.
    ///
    /// # Invariant SC-3 / SC-4
    pub active_ms_today: Mutex<u64>,

    // ── Timer handles (Phase 4A+) ─────────────────────────────────────────────

    /// Active reminder timer abort handle.
    ///
    /// `Some(handle)` while the countdown is running or paused.
    /// `None` when the scheduler is stopped.
    /// Replaced (aborted then re-created) on every restart.
    ///
    /// # Invariant T-1
    pub reminder_timer: Mutex<Option<AbortHandle>>,

    /// Active snooze timer abort handle.
    ///
    /// Populated when the user snoozes a triggered session.
    /// `None` at all other times.
    ///
    /// # Invariant T-2 / T-3
    pub snooze_timer: Mutex<Option<AbortHandle>>,

    /// Active session-timeout timer abort handle.
    ///
    /// Populated when a triggered session starts its auto-timeout countdown.
    /// `None` at all other times.
    ///
    /// # Invariant T-4
    pub timeout_timer: Mutex<Option<AbortHandle>>,
}

impl AppState {
    /// Creates a new `AppState` with the given database pool and initial settings.
    ///
    /// All timer and scheduler fields start in their "inactive" state.
    pub fn new(db: Pool<Sqlite>, settings: AppSettings) -> Self {
        Self {
            db,
            settings: Mutex::new(settings),
            active_session_id: Mutex::new(None),
            scheduler_paused: Mutex::new(false),
            remaining_ms: Mutex::new(None),
            active_ms_today: Mutex::new(0),
            reminder_timer: Mutex::new(None),
            snooze_timer: Mutex::new(None),
            timeout_timer: Mutex::new(None),
        }
    }
}
