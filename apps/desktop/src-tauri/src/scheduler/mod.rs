//! Reminder Scheduler — Phase 4A.
//!
//! Determines WHEN a Hydration Session should be triggered.
//! Does NOT determine what happens after triggering (that is the Reminder Engine, Phase 4B+).
//!
//! # State machine (Architecture §3 / State Machine §2)
//!
//! ```text
//!   Stopped ──start()──► Running ──timer expires──► Triggered
//!      ▲                    │                           │
//!      └──────stop()────────┘         (explicit restart required)
//!                           │
//!                       pause()
//!                           │
//!                           ▼
//!                        Paused ──resume()──► Running
//! ```
//!
//! # Thread safety
//!
//! All mutations go through `AppState` fields protected by `Mutex`.
//! `AbortHandle` is `Send + Sync`; the spawned task owns the sleep future.
//!
//! # Invariant T-1
//! Only one reminder timer task may exist at any time.
//! Every `start()` / `resume()` aborts any existing handle before spawning a new one.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::task::AbortHandle;

use crate::errors::{AppError, AppResult};
use crate::services::SessionService;
use crate::state::AppState;

// ── Scheduler state enum ──────────────────────────────────────────────────────

/// All valid states for the Reminder Scheduler.
///
/// Transitions are enforced by each public function.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerState {
    /// No timer is running. The scheduler has never been started, or was explicitly stopped.
    Stopped,
    /// A countdown timer is running. A reminder will fire when it expires.
    Running,
    /// The timer has been paused. `remaining_ms` in `AppState` holds the preserved duration.
    Paused,
    /// The timer has fired. A pending `HydrationSession` has been created and an event emitted.
    /// The scheduler does NOT auto-restart from this state (explicit `start()` required).
    Triggered,
}

impl std::fmt::Display for SchedulerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped   => write!(f, "stopped"),
            Self::Running   => write!(f, "running"),
            Self::Paused    => write!(f, "paused"),
            Self::Triggered => write!(f, "triggered"),
        }
    }
}

// ── Internal event payload ────────────────────────────────────────────────────

/// Emitted internally on `"aquatick://reminder/triggered"` when the timer fires.
/// The Reminder Engine (Phase 4B) listens for this event to drive the character
/// animation and notification display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReminderTriggeredPayload {
    /// The ID of the `HydrationSession` row just created in SQLite.
    pub session_id: i64,
    /// Snapshot of `character_id` from settings at the moment of triggering.
    pub character_id: String,
    /// Snapshot of `sound_id` from settings at the moment of triggering.
    /// `None` when sound is disabled.
    pub sound_id: Option<String>,
    /// Unix ms when the timer fired.
    pub triggered_at: i64,
}

// ── Scheduler service ─────────────────────────────────────────────────────────

/// Reminder Scheduler — manages the countdown timer that creates Hydration Sessions.
pub struct SchedulerService;

impl SchedulerService {
    // ── Timer cancellation helper ─────────────────────────────────────────────

    /// Aborts and clears the existing `reminder_timer` handle in `AppState`.
    ///
    /// Safe to call even when no timer is running (`None` is a no-op).
    fn cancel_existing(state: &AppState) {
        if let Ok(mut guard) = state.reminder_timer.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Starts the reminder countdown for the full configured interval.
    ///
    /// # Behaviour
    /// - Aborts any existing timer (Invariant T-1).
    /// - Clears `scheduler_paused` and `remaining_ms`.
    /// - Spawns a new tokio task that sleeps for `interval_ms`.
    /// - On wake: creates a pending `HydrationSession` and emits `"aquatick://reminder/triggered"`.
    ///
    /// # Errors
    /// Returns `AppError::Validation` if `interval_ms` is zero.
    pub async fn start(
        app_handle: AppHandle,
        state: &AppState,
        interval_ms: u64,
    ) -> AppResult<()> {
        if interval_ms == 0 {
            return Err(AppError::Validation(
                "Scheduler interval must be greater than zero".to_string(),
            ));
        }

        // Abort any previous timer (Invariant T-1)
        Self::cancel_existing(state);

        // Clear paused flag and preserved remaining time
        if let Ok(mut p) = state.scheduler_paused.lock() {
            *p = false;
        }
        if let Ok(mut r) = state.remaining_ms.lock() {
            *r = None;
        }

        let handle = Self::spawn_timer(app_handle, state, interval_ms)?;

        if let Ok(mut guard) = state.reminder_timer.lock() {
            *guard = Some(handle);
        }

        log::info!("Scheduler started. Interval: {}ms ({:.1}min).", interval_ms, interval_ms as f64 / 60_000.0);
        Ok(())
    }

    /// Stops the scheduler completely. The state transitions to `Stopped`.
    ///
    /// After stopping, `start()` must be called explicitly to resume scheduling.
    pub fn stop(state: &AppState) {
        Self::cancel_existing(state);

        if let Ok(mut p) = state.scheduler_paused.lock() {
            *p = false;
        }
        if let Ok(mut r) = state.remaining_ms.lock() {
            *r = None;
        }

        log::info!("Scheduler stopped.");
    }

    /// Pauses the scheduler, preserving the remaining duration so it can be resumed exactly.
    ///
    /// # Behaviour
    /// - Aborts the running timer task.
    /// - Records `remaining_ms` based on `started_at` and `interval_ms`.
    /// - Sets `scheduler_paused = true`.
    ///
    /// The caller must supply `started_at` (the `Instant` when the current timer began)
    /// and the original `interval_ms` so remaining time can be computed.
    pub fn pause(state: &AppState, started_at: Instant, interval_ms: u64) {
        let elapsed_ms = started_at.elapsed().as_millis() as u64;
        let remaining = interval_ms.saturating_sub(elapsed_ms);

        Self::cancel_existing(state);

        if let Ok(mut r) = state.remaining_ms.lock() {
            *r = Some(remaining);
        }
        if let Ok(mut p) = state.scheduler_paused.lock() {
            *p = true;
        }

        log::info!(
            "Scheduler paused. Remaining: {}ms ({:.1}min).",
            remaining,
            remaining as f64 / 60_000.0
        );
    }

    /// Resumes the scheduler from the preserved `remaining_ms` stored in `AppState`.
    ///
    /// # Errors
    /// Returns `AppError::Validation` if no remaining duration is stored (scheduler was not paused).
    pub async fn resume(app_handle: AppHandle, state: &AppState) -> AppResult<()> {
        let remaining_ms = {
            let guard = state.remaining_ms.lock().map_err(|_| {
                AppError::Internal("Failed to lock remaining_ms".to_string())
            })?;
            (*guard).ok_or_else(|| {
                AppError::Validation(
                    "Cannot resume scheduler: no remaining duration stored. \
                     Was the scheduler paused?"
                        .to_string(),
                )
            })?
        };

        // Abort any existing timer (should be None after pause, but guard for safety)
        Self::cancel_existing(state);

        let handle = Self::spawn_timer(app_handle, state, remaining_ms)?;

        if let Ok(mut guard) = state.reminder_timer.lock() {
            *guard = Some(handle);
        }
        if let Ok(mut p) = state.scheduler_paused.lock() {
            *p = false;
        }
        if let Ok(mut r) = state.remaining_ms.lock() {
            *r = None;
        }

        log::info!(
            "Scheduler resumed. Remaining: {}ms ({:.1}min).",
            remaining_ms,
            remaining_ms as f64 / 60_000.0
        );
        Ok(())
    }

    /// Resets the scheduler to `Stopped` state and immediately starts a fresh countdown.
    ///
    /// Equivalent to `stop()` followed by `start()`.
    pub async fn reset(
        app_handle: AppHandle,
        state: &AppState,
        interval_ms: u64,
    ) -> AppResult<()> {
        Self::stop(state);
        Self::start(app_handle, state, interval_ms).await?;
        log::info!("Scheduler reset with interval {}ms.", interval_ms);
        Ok(())
    }

    /// Returns a snapshot of the current scheduler logical state.
    ///
    /// This is a best-effort query — it reads `Mutex` fields without blocking.
    pub fn status(state: &AppState) -> SchedulerState {
        let paused = state.scheduler_paused.lock().map(|g| *g).unwrap_or(false);
        let has_timer = state.reminder_timer.lock().map(|g| g.is_some()).unwrap_or(false);

        if paused {
            SchedulerState::Paused
        } else if has_timer {
            SchedulerState::Running
        } else {
            SchedulerState::Stopped
        }
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    /// Spawns the timer task and returns its `AbortHandle`.
    ///
    /// When the sleep completes (not aborted), the task:
    /// 1. Reads current settings from `AppState`.
    /// 2. Creates a `pending` HydrationSession via `SessionService::create_pending`.
    /// 3. Records the session ID in `AppState.active_session_id`.
    /// 4. Clears the `reminder_timer` handle (task is done).
    /// 5. Emits `"aquatick://reminder/triggered"` with a `ReminderTriggeredPayload`.
    fn spawn_timer(
        app_handle: AppHandle,
        state: &AppState,
        duration_ms: u64,
    ) -> AppResult<AbortHandle> {
        // Snapshot the database pool and settings needed inside the spawned task.
        let pool = state.db.clone();

        let (character_id, sound_id, interval_minutes) = {
            let s = state.settings.lock().map_err(|_| {
                AppError::Internal("Failed to lock settings".to_string())
            })?;
            let sid = if s.sound_enabled {
                // Sound file ID matches the character slug — Reminder Engine resolves the path.
                Some(format!("{}_drink", s.character_id))
            } else {
                None
            };
            (s.character_id.clone(), sid, s.reminder_interval_minutes)
        };

        // Clone AppHandle — it is internally Arc-wrapped in Tauri v2 so this is cheap.
        let app_handle_clone = app_handle.clone();

        let join_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(duration_ms)).await;

            // Retrieve the managed AppState from the AppHandle
            let state_ref: tauri::State<'_, AppState> = app_handle_clone.state();

            // Check if scheduler was paused or stopped while we were sleeping
            let paused = state_ref.scheduler_paused.lock().map(|g| *g).unwrap_or(false);
            if paused {
                log::info!("Scheduler timer fired but scheduler is paused — session not created.");
                return;
            }

            let now_ms = chrono::Utc::now().timestamp_millis();

            // Create the pending HydrationSession
            match SessionService::create_pending(
                &pool,
                now_ms,
                interval_minutes,
                character_id.clone(),
                sound_id.clone(),
            )
            .await
            {
                Ok(session) => {
                    // Record the active session ID in AppState
                    if let Ok(mut guard) = state_ref.active_session_id.lock() {
                        *guard = Some(session.id);
                    }

                    // Clear the reminder_timer handle — this task is done
                    if let Ok(mut guard) = state_ref.reminder_timer.lock() {
                        *guard = None;
                    }

                    log::info!(
                        "Reminder triggered. Created HydrationSession id={} at {}ms.",
                        session.id,
                        now_ms
                    );

                    // Emit the internal reminder event
                    let payload = ReminderTriggeredPayload {
                        session_id: session.id,
                        character_id: character_id.clone(),
                        sound_id: sound_id.clone(),
                        triggered_at: now_ms,
                    };

                    if let Err(e) = app_handle_clone.emit("aquatick://reminder/triggered", payload) {
                        log::error!("Failed to emit reminder/triggered event: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to create HydrationSession on timer fire: {}", e);
                    // Clear the handle so the scheduler shows as Stopped (error recovery)
                    if let Ok(mut guard) = state_ref.reminder_timer.lock() {
                        *guard = None;
                    }
                }
            }
        });

        Ok(join_handle.abort_handle())
    }
}
