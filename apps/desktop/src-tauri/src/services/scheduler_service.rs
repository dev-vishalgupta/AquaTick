//! Internal scheduler for determining when a hydration reminder should fire.
//!
//! # Scope
//!
//! This module is a **pure backend component**. It has zero knowledge of:
//! - Tauri / IPC / events
//! - React / frontend / UI
//! - Characters, sounds, animations
//! - Idle detection
//! - Snooze logic
//! - Session timeout handling
//!
//! Its only external dependency is `SessionService::create_pending`.
//!
//! # Scheduler State Machine (State Machine §2, simplified)
//!
//! ```text
//!   ┌─────────┐  start(interval)  ┌─────────┐
//!   │ Stopped │ ────────────────► │ Running │
//!   └─────────┘                   └────┬────┘
//!       ▲                              │ pause()
//!       │ stop()                  ┌────▼────┐
//!       │                         │ Paused  │
//!       │                         └────┬────┘
//!       │                              │ resume()
//!       │                         ┌────▼────┐
//!       │         stop()          │ Running │ (resumes from remaining)
//!       │    ◄─────────────────── └────┬────┘
//!       │                              │ timer expires
//!       │                         ┌────▼──────┐
//!       └──────── stop() ─────── │ Triggered │
//!                                 └───────────┘
//! ```
//!
//! # Thread Safety
//!
//! `SchedulerService` wraps all mutable state in `Arc<Mutex<_>>`. The timer task
//! holds a clone of the same `Arc`, so both the control surface and the timer
//! task see the same state without data races.
//!
//! # Invariant T-1
//!
//! Only one timer task may exist at any time. This is enforced by storing the
//! current task's `AbortHandle` inside `InnerState` and calling `abort()` before
//! spawning a new task.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::{Pool, Sqlite};
use tokio::task::AbortHandle;

use crate::services::SessionService;

// ── Scheduler state ───────────────────────────────────────────────────────────

/// The four states of the scheduler state machine.
///
/// Transitions must follow the approved state machine diagram (State Machine §2).
/// Invalid transitions are silently no-ops — the scheduler logs the rejection
/// but does not panic.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerState {
    /// No timer is running. Initial state and state after `stop()`.
    Stopped,
    /// Timer is actively counting down toward the next session trigger.
    Running,
    /// Timer is paused. Remaining duration is preserved in `InnerState::remaining`.
    Paused,
    /// Timer has expired. One pending session has been created. Awaiting external handling.
    Triggered,
}

impl std::fmt::Display for SchedulerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped   => write!(f, "Stopped"),
            Self::Running   => write!(f, "Running"),
            Self::Paused    => write!(f, "Paused"),
            Self::Triggered => write!(f, "Triggered"),
        }
    }
}

// ── Inner mutable state ───────────────────────────────────────────────────────

/// All mutable fields of the scheduler, protected by a single `Mutex`.
///
/// Never exposed publicly — callers interact only through `SchedulerService`.
struct InnerState {
    /// Current scheduler state.
    state: SchedulerState,

    /// The full configured interval for a fresh timer.
    interval: Duration,

    /// Remaining duration at the time the scheduler was last paused.
    ///
    /// `None` when running from a full interval or when stopped.
    /// `Some(d)` when paused, carrying the un-elapsed portion.
    remaining: Option<Duration>,

    /// Instant when the current running segment began.
    ///
    /// Used to calculate remaining time when `pause()` is called.
    /// `None` when not in the `Running` state.
    started_at: Option<Instant>,

    /// Handle to cancel the active timer task.
    ///
    /// Invariant T-1: always cancelled before a new timer is spawned.
    timer_handle: Option<AbortHandle>,
}

impl InnerState {
    fn new() -> Self {
        Self {
            state:        SchedulerState::Stopped,
            interval:     Duration::ZERO,
            remaining:    None,
            started_at:   None,
            timer_handle: None,
        }
    }

    /// Cancels the active timer task if one exists. Idempotent.
    fn cancel_timer(&mut self) {
        if let Some(handle) = self.timer_handle.take() {
            handle.abort();
        }
    }
}

// ── SchedulerService ──────────────────────────────────────────────────────────

/// Internal reminder scheduler.
///
/// `SchedulerService` is `Clone` and `Send + Sync` — it wraps all mutable state
/// in an `Arc<Mutex<_>>` and can be safely shared across threads.
///
/// # Usage
///
/// ```rust,ignore
/// let scheduler = SchedulerService::new();
/// scheduler.start(pool.clone(), interval, settings).await;
/// // … later …
/// scheduler.pause().await;
/// scheduler.resume().await;
/// scheduler.stop().await;
/// ```
#[derive(Clone)]
pub struct SchedulerService {
    inner: Arc<Mutex<InnerState>>,
}

impl SchedulerService {
    /// Creates a new scheduler instance in the `Stopped` state.
    ///
    /// The caller (usually `AppState`) owns the single canonical instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState::new())),
        }
    }

    /// Returns the current scheduler state.
    ///
    /// Does not block for longer than the time needed to lock `inner`.
    pub fn state(&self) -> SchedulerState {
        self.inner
            .lock()
            .map(|g| g.state.clone())
            .unwrap_or(SchedulerState::Stopped)
    }

    /// Returns the remaining duration left on the current timer, if applicable.
    ///
    /// - `Running`: live remaining = configured remaining minus elapsed since `started_at`.
    /// - `Paused`: the captured remaining at the moment of pause.
    /// - `Stopped` / `Triggered`: `None`.
    pub fn remaining(&self) -> Option<Duration> {
        let guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(_) => return None,
        };

        match guard.state {
            SchedulerState::Running => {
                let elapsed = guard.started_at
                    .map(|t| t.elapsed())
                    .unwrap_or(Duration::ZERO);

                let base = guard.remaining.unwrap_or(guard.interval);
                Some(base.saturating_sub(elapsed))
            }
            SchedulerState::Paused => guard.remaining,
            _ => None,
        }
    }

    // ── State transitions ─────────────────────────────────────────────────────

    /// Starts the scheduler with the given interval.
    ///
    /// Valid from: `Stopped`, `Triggered`.
    /// Invalid from: `Running`, `Paused` (logged and ignored).
    ///
    /// Cancels any existing timer before spawning a fresh one.
    pub async fn start(
        &self,
        pool: Pool<Sqlite>,
        interval: Duration,
        character_id: String,
        sound_id: Option<String>,
        interval_minutes: i64,
    ) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("Scheduler: failed to acquire lock in start(): {}", e);
                return;
            }
        };

        match guard.state {
            SchedulerState::Running | SchedulerState::Paused => {
                log::warn!(
                    "Scheduler: start() called while in state '{}' — ignoring.",
                    guard.state
                );
                return;
            }
            _ => {}
        }

        // Cancel any lingering timer from the previous cycle.
        guard.cancel_timer();

        guard.interval  = interval;
        guard.remaining = None;
        guard.state     = SchedulerState::Running;
        guard.started_at = Some(Instant::now());

        let handle = self.spawn_timer(
            pool,
            interval,
            character_id,
            sound_id,
            interval_minutes,
        );
        guard.timer_handle = Some(handle);

        log::info!("Scheduler: Started with interval {:?}.", interval);
    }

    /// Stops the scheduler unconditionally and resets all state.
    ///
    /// Valid from any state.
    pub async fn stop(&self) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("Scheduler: failed to acquire lock in stop(): {}", e);
                return;
            }
        };

        guard.cancel_timer();
        guard.state      = SchedulerState::Stopped;
        guard.remaining  = None;
        guard.started_at = None;

        log::info!("Scheduler: Stopped.");
    }

    /// Pauses the scheduler, capturing the remaining time.
    ///
    /// Valid from: `Running`.
    /// Invalid from: `Stopped`, `Paused`, `Triggered` (logged and ignored).
    pub async fn pause(&self) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("Scheduler: failed to acquire lock in pause(): {}", e);
                return;
            }
        };

        if guard.state != SchedulerState::Running {
            log::warn!(
                "Scheduler: pause() called while in state '{}' — ignoring.",
                guard.state
            );
            return;
        }

        // Capture remaining time before cancelling the task.
        let elapsed = guard.started_at.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
        let base    = guard.remaining.unwrap_or(guard.interval);
        let remaining = base.saturating_sub(elapsed);

        guard.cancel_timer();
        guard.remaining  = Some(remaining);
        guard.started_at = None;
        guard.state      = SchedulerState::Paused;

        log::info!("Scheduler: Paused. Remaining: {:?}.", remaining);
    }

    /// Resumes the scheduler from the captured remaining time.
    ///
    /// Valid from: `Paused`.
    /// Invalid from: `Stopped`, `Running`, `Triggered` (logged and ignored).
    pub async fn resume(
        &self,
        pool: Pool<Sqlite>,
        character_id: String,
        sound_id: Option<String>,
        interval_minutes: i64,
    ) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("Scheduler: failed to acquire lock in resume(): {}", e);
                return;
            }
        };

        if guard.state != SchedulerState::Paused {
            log::warn!(
                "Scheduler: resume() called while in state '{}' — ignoring.",
                guard.state
            );
            return;
        }

        let remaining = guard.remaining.unwrap_or(guard.interval);
        guard.state      = SchedulerState::Running;
        guard.started_at = Some(Instant::now());

        let handle = self.spawn_timer(
            pool,
            remaining,
            character_id,
            sound_id,
            interval_minutes,
        );
        guard.timer_handle = Some(handle);

        log::info!("Scheduler: Resumed. Firing in {:?}.", remaining);
    }

    /// Resets the scheduler: stops any running timer and restarts from the full interval.
    ///
    /// Valid from: `Running`, `Paused`, `Triggered`.
    /// No-op from `Stopped`.
    pub async fn reset(
        &self,
        pool: Pool<Sqlite>,
        character_id: String,
        sound_id: Option<String>,
        interval_minutes: i64,
    ) {
        // Capture current interval before stopping (stop() clears state).
        let interval = {
            match self.inner.lock() {
                Ok(g)  => g.interval,
                Err(e) => {
                    log::error!("Scheduler: failed to acquire lock in reset(): {}", e);
                    return;
                }
            }
        };

        // Full stop.
        self.stop().await;

        // Re-start only if we had a meaningful interval configured.
        if interval > Duration::ZERO {
            self.start(pool, interval, character_id, sound_id, interval_minutes).await;
            log::info!("Scheduler: Reset — restarted from full interval {:?}.", interval);
        } else {
            log::warn!("Scheduler: reset() called with zero interval — staying Stopped.");
        }
    }

    // ── Private timer ─────────────────────────────────────────────────────────

    /// Spawns a tokio task that sleeps for `duration` then fires the trigger logic.
    ///
    /// Returns the `AbortHandle` so the caller can cancel it on `stop()` or `pause()`.
    ///
    /// # On expiry
    ///
    /// 1. Creates one pending `HydrationSession` via `SessionService::create_pending`.
    /// 2. Transitions the scheduler state to `Triggered`.
    ///
    /// Nothing else. No events, no IPC, no character selection, no sound, no UI.
    fn spawn_timer(
        &self,
        pool: Pool<Sqlite>,
        duration: Duration,
        character_id: String,
        sound_id: Option<String>,
        interval_minutes: i64,
    ) -> AbortHandle {
        let inner = Arc::clone(&self.inner);

        let task = tokio::spawn(async move {
            tokio::time::sleep(duration).await;

            // At this point the task has NOT been aborted — the duration elapsed.
            // Create the pending session.
            let now_ms = chrono::Utc::now().timestamp_millis();

            match SessionService::create_pending(
                &pool,
                now_ms,           // scheduled_at = now (the moment it fires)
                interval_minutes,
                character_id,
                sound_id,
            )
            .await
            {
                Ok(session) => {
                    log::info!(
                        "Scheduler: Reminder triggered. Created pending session ID {}.",
                        session.id
                    );

                    // Transition scheduler state to Triggered.
                    match inner.lock() {
                        Ok(mut g) => {
                            g.timer_handle = None; // task is finishing; handle is moot
                            g.started_at   = None;
                            g.remaining    = None;
                            g.state        = SchedulerState::Triggered;
                        }
                        Err(e) => {
                            log::error!(
                                "Scheduler: failed to transition to Triggered state: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Scheduler: failed to create pending session on timer expiry: {}",
                        e
                    );
                    // Do not transition to Triggered — the session wasn't created.
                    // Leave state as Running so the caller can decide what to do.
                }
            }
        });

        task.abort_handle()
    }
}

impl Default for SchedulerService {
    fn default() -> Self {
        Self::new()
    }
}
