//! Shared application state.
//!
//! `AppState` is registered with Tauri via `app.manage(AppState::new(...))` in
//! `lib.rs`. Command handlers receive it as `tauri::State<'_, AppState>`.
//!
//! # Field ownership rules (Architecture §4, Invariants §8)
//!
//! | Field                | Owner    | Description                                        |
//! |----------------------|----------|----------------------------------------------------|
//! | `db`                 | Phase 3A | Live database pool                                 |
//! | `settings`           | Phase 3A | Eagerly-loaded settings cache                      |
//! | `active_session_id`  | Phase 3E | ID of the currently open Hydration Session         |
//! | `scheduler_paused`   | Phase 3E | Active Usage SM — scheduler paused flag            |
//! | `remaining_ms`       | Phase 3E | Timer preservation across pause/resume             |
//! | `active_ms_today`    | Phase 3E | Usage time accumulation for the current day        |
//! | `scheduler`          | Phase 4A | Internal reminder scheduler (SchedulerService)     |
//! | `activity_monitor`   | Phase 4B | Active Usage Monitor (ActivityMonitorService)      |
//! | `reminder_engine`    | Phase 4C | Reminder lifecycle orchestrator (ReminderEngine)   |

use sqlx::{Pool, Sqlite};
use std::sync::Mutex;

use crate::models::AppSettings;
use crate::services::{ActivityMonitorService, ReminderEngineService, SchedulerService};

/// Tauri-managed shared state for the AquaTick backend.
///
/// All mutable fields are wrapped in `Mutex` so they can be safely accessed
/// from concurrent Tauri command handlers.
pub struct AppState {
    // ── Database ──────────────────────────────────────────────────────────────

    /// SQLite connection pool — the only database access point in the backend.
    ///
    /// `Pool<Sqlite>` is internally `Arc`-wrapped and clone-able.
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
    /// Set to `Some(id)` when the scheduler transitions to `Triggered`.
    /// Cleared to `None` when a session reaches `completed` or `timed_out`.
    ///
    /// # Invariant S-1 / S-6
    /// Only one active session may exist at any time. A snoozed session is
    /// still considered active (`Some(id)` is preserved during snooze).
    pub active_session_id: Mutex<Option<i64>>,

    // ── Active Usage tracking ─────────────────────────────────────────────────

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
    /// Used to compute `daily_statistics.expected_sessions`:
    ///   `expected = active_ms_today / (interval_minutes * 60 * 1000)`
    ///
    /// Resets at midnight. Written to `daily_statistics` when a session resolves.
    ///
    /// # Invariant SC-3 / SC-4
    pub active_ms_today: Mutex<u64>,

    // ── Scheduler (Phase 4A) ──────────────────────────────────────────────────

    /// Internal reminder scheduler.
    ///
    /// Owns the single timer task (`AbortHandle`) and the scheduler state machine.
    /// All timing and session-creation on expiry is encapsulated inside this struct.
    ///
    /// # Invariant T-1
    /// `SchedulerService` ensures only one timer task exists at any time by
    /// cancelling the previous `AbortHandle` before spawning a new one.
    pub scheduler: SchedulerService,

    // ── Activity Monitor (Phase 4B) ───────────────────────────────────────────

    /// Active Usage Monitor.
    ///
    /// Detects user idle, system sleep/wake, and screen lock events.
    /// Calls `SchedulerService::pause()` / `SchedulerService::resume()` in response.
    ///
    /// # Invariant AM-1
    /// Only one poll task may exist at any time. `ActivityMonitorService::start()`
    /// is a no-op if the monitor is already running.
    pub activity_monitor: ActivityMonitorService,

    // ── Reminder Engine (Phase 4C) ────────────────────────────────────────────

    /// Reminder lifecycle orchestrator.
    ///
    /// Owns the session creation, state transitions, timeout timer, and snooze
    /// timer. The scheduler calls the engine via a stored callback when the
    /// reminder interval expires.
    ///
    /// # Invariant RE-1
    /// Only one active `HydrationSession` may exist at any time.
    /// # Invariant RE-2
    /// Only one timeout timer may exist at any time.
    /// # Invariant RE-3
    /// Only one snooze timer may exist at any time.
    pub reminder_engine: ReminderEngineService,
}

impl AppState {
    /// Creates a new `AppState` with the given database pool and initial settings.
    ///
    /// Both the scheduler and the activity monitor start in their respective
    /// idle states (`Stopped` and `Active`). No background tasks are running
    /// until explicitly started.
    pub fn new(db: Pool<Sqlite>, settings: AppSettings) -> Self {
        Self {
            db,
            settings:          Mutex::new(settings),
            active_session_id: Mutex::new(None),
            scheduler_paused:  Mutex::new(false),
            remaining_ms:      Mutex::new(None),
            active_ms_today:   Mutex::new(0),
            scheduler:         SchedulerService::new(),
            activity_monitor:  ActivityMonitorService::new(),
            reminder_engine:   ReminderEngineService::new(),
        }
    }
}
