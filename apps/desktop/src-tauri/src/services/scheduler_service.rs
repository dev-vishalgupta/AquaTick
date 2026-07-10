//! Internal scheduler for determining when a hydration reminder should fire.
//!
//! # Scope
//!
//! This module is a **pure time-keeping component**. It has zero knowledge of:
//! - Tauri / IPC / events
//! - React / frontend / UI
//! - Characters, sounds, animations
//! - Idle detection
//! - Snooze logic
//! - Session timeout handling
//! - Hydration sessions (creation or management)
//!
//! Its **only responsibility** is counting down an interval and notifying the
//! `ReminderEngineService` when the interval expires via the `on_reminder_due`
//! callback. Session creation is entirely owned by the Reminder Engine.
//!
//! # Scheduler State Machine (State Machine §2, simplified)
//!
//! ```text
//!   +---------+  start(interval)  +---------+
//!   ¦ Stopped ¦ ----------------? ¦ Running ¦
//!   +---------+                   +---------+
//!       ?                              ¦ pause()
//!       ¦ stop()                  +----?----+
//!       ¦                         ¦ Paused  ¦
//!       ¦                         +---------+
//!       ¦                              ¦ resume()
//!       ¦                         +----?----+
//!       ¦         stop()          ¦ Running ¦ (resumes from remaining)
//!       ¦    ?------------------- +---------+
//!       ¦                              ¦ timer expires
//!       ¦                         +----?------+
//!       +-------- stop() ------- ¦ Triggered ¦
//!                                 +-----------+
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
//! current task`s `AbortHandle` inside `InnerState` and calling `abort()` before
//! spawning a new task.
//!
//! # Invariant T-2 (Guard Check)
//!
//! After the timer sleep completes, the scheduler re-acquires the lock and
//! verifies the state is still `Running` before invoking the callback. This
//! prevents the reminder-due callback from firing if `pause()` or `stop()`
//! raced with the timer expiry.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::task::AbortHandle;

// -- Scheduler state -----------------------------------------------------------

/// The four states of the scheduler state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerState {
    Stopped,
    Running,
    Paused,
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

// -- Callback type -------------------------------------------------------------

/// Callback invoked exactly once when the reminder timer fires.
///
/// The `ReminderEngineService` registers this callback so it can take over
/// the reminder lifecycle without the scheduler needing session knowledge.
pub type OnReminderDue = Arc<dyn Fn() + Send + Sync + 'static>;

// -- Inner mutable state -------------------------------------------------------

struct InnerState {
    state:           SchedulerState,
    interval:        Duration,
    remaining:       Option<Duration>,
    started_at:      Option<Instant>,
    timer_handle:    Option<AbortHandle>,
    on_reminder_due: Option<OnReminderDue>,
}

impl InnerState {
    fn new() -> Self {
        Self {
            state:           SchedulerState::Stopped,
            interval:        Duration::ZERO,
            remaining:       None,
            started_at:      None,
            timer_handle:    None,
            on_reminder_due: None,
        }
    }

    fn cancel_timer(&mut self) {
        if let Some(handle) = self.timer_handle.take() {
            handle.abort();
        }
    }
}

// -- SchedulerService ----------------------------------------------------------

/// Internal reminder scheduler — owns only the countdown timer.
#[derive(Clone)]
pub struct SchedulerService {
    inner: Arc<Mutex<InnerState>>,
}

impl SchedulerService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState::new())),
        }
    }

    pub fn state(&self) -> SchedulerState {
        self.inner
            .lock()
            .map(|g| g.state.clone())
            .unwrap_or(SchedulerState::Stopped)
    }

    pub fn remaining(&self) -> Option<Duration> {
        let guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(_) => return None,
        };
        match guard.state {
            SchedulerState::Running => {
                let elapsed = guard.started_at.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
                let base = guard.remaining.unwrap_or(guard.interval);
                Some(base.saturating_sub(elapsed))
            }
            SchedulerState::Paused => guard.remaining,
            _ => None,
        }
    }

    /// Start the scheduler with a fresh interval and a reminder-due callback.
    ///
    /// Valid from: `Stopped`, `Triggered`.
    /// No-op from `Running` or `Paused`.
    pub async fn start(&self, interval: Duration, on_reminder_due: OnReminderDue) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => { log::error!("Scheduler: lock failed in start(): {}", e); return; }
        };

        match guard.state {
            SchedulerState::Running | SchedulerState::Paused => {
                log::warn!("Scheduler: start() in state '{}' — ignoring.", guard.state);
                return;
            }
            _ => {}
        }

        guard.cancel_timer();
        guard.on_reminder_due = Some(on_reminder_due.clone());
        guard.interval        = interval;
        guard.remaining       = None;
        guard.state           = SchedulerState::Running;
        guard.started_at      = Some(Instant::now());

        let handle = self.spawn_timer(interval, on_reminder_due);
        guard.timer_handle = Some(handle);
        log::info!("Scheduler: Started with interval {:?}.", interval);
    }

    /// Stop the scheduler unconditionally.
    pub async fn stop(&self) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => { log::error!("Scheduler: lock failed in stop(): {}", e); return; }
        };
        guard.cancel_timer();
        guard.state      = SchedulerState::Stopped;
        guard.remaining  = None;
        guard.started_at = None;
        log::info!("Scheduler: Stopped.");
    }

    /// Pause the scheduler, capturing remaining time.
    ///
    /// Valid from: `Running` only.
    pub async fn pause(&self) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => { log::error!("Scheduler: lock failed in pause(): {}", e); return; }
        };

        if guard.state != SchedulerState::Running {
            log::warn!("Scheduler: pause() in state '{}' — ignoring.", guard.state);
            return;
        }

        let elapsed   = guard.started_at.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
        let base      = guard.remaining.unwrap_or(guard.interval);
        let remaining = base.saturating_sub(elapsed);

        guard.cancel_timer();
        guard.remaining  = Some(remaining);
        guard.started_at = None;
        guard.state      = SchedulerState::Paused;
        log::info!("Scheduler: Paused. Remaining: {:?}.", remaining);
    }

    /// Resume from the captured remaining time. Idempotent from non-Paused states.
    ///
    /// Valid from: `Paused` only.
    pub async fn resume(&self) {
        let (remaining, callback) = {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => { log::error!("Scheduler: lock failed in resume(): {}", e); return; }
            };

            if guard.state != SchedulerState::Paused {
                log::warn!("Scheduler: resume() in state '{}' — ignoring.", guard.state);
                return;
            }

            let callback = match guard.on_reminder_due.clone() {
                Some(cb) => cb,
                None => {
                    log::error!("Scheduler: resume() — no callback; was start() called?");
                    return;
                }
            };

            let remaining = guard.remaining.unwrap_or(guard.interval);
            guard.state      = SchedulerState::Running;
            guard.started_at = Some(Instant::now());

            let handle = self.spawn_timer(remaining, callback.clone());
            guard.timer_handle = Some(handle);

            (remaining, callback)
        };

        let _ = callback;
        log::info!("Scheduler: Resumed. Firing in {:?}.", remaining);
    }

    /// Reset to the full interval using the stored callback.
    pub async fn reset(&self) {
        let (interval, callback) = {
            match self.inner.lock() {
                Ok(g)  => (g.interval, g.on_reminder_due.clone()),
                Err(e) => { log::error!("Scheduler: lock failed in reset(): {}", e); return; }
            }
        };

        if interval == Duration::ZERO {
            log::warn!("Scheduler: reset() with zero interval — staying Stopped.");
            return;
        }
        let callback = match callback {
            Some(cb) => cb,
            None => { log::warn!("Scheduler: reset() — no callback stored."); return; }
        };

        self.stop().await;
        self.start(interval, callback).await;
        log::info!("Scheduler: Reset — restarted from full interval {:?}.", interval);
    }

    // -- Private ---------------------------------------------------------------

    fn spawn_timer(&self, duration: Duration, on_reminder_due: OnReminderDue) -> AbortHandle {
        let inner = Arc::clone(&self.inner);

        let task = tokio::spawn(async move {
            tokio::time::sleep(duration).await;

            // Guard check T-2: verify state is still Running before firing.
            {
                let mut guard = match inner.lock() {
                    Ok(g)  => g,
                    Err(e) => { log::error!("Scheduler: lock failed on expiry: {}", e); return; }
                };

                if guard.state != SchedulerState::Running {
                    log::info!(
                        "Scheduler: Timer fired but state is '{}' — discarding (T-2).",
                        guard.state
                    );
                    return;
                }

                // Transition to Triggered while holding the lock.
                guard.timer_handle = None;
                guard.started_at   = None;
                guard.remaining    = None;
                guard.state        = SchedulerState::Triggered;
            }

            log::info!("Scheduler: Reminder due — invoking on_reminder_due callback.");
            on_reminder_due();
        });

        task.abort_handle()
    }
}

impl Default for SchedulerService {
    fn default() -> Self { Self::new() }
}
