//! Active Usage Monitor — determines whether the user is currently present.
//!
//! # Scope
//!
//! This module is a **pure backend component**. It has zero knowledge of:
//! - Tauri / IPC / Tauri events
//! - React / frontend / UI
//! - Hydration sessions
//! - Characters, sounds, animations
//! - Snooze or timeout logic
//! - Statistics
//!
//! Its **only external responsibility** is calling `SchedulerService::pause()`
//! and `SchedulerService::resume()`. It never manipulates timers directly.
//!
//! # Monitor State Machine (State Machine §3)
//!
//! ```text
//!   ┌────────┐  idle timeout  ┌──────┐
//!   │ Active │ ─────────────► │ Idle │
//!   └───┬────┘                └──┬───┘
//!       │ ◄── activity ──────────┘
//!       │
//!       │  sleep gap        ┌──────────┐
//!       ├──────────────────► Sleeping  │
//!       │                   └────┬─────┘
//!       │ ◄── wake (same tick) ──┘
//!       │
//!       │  [platform signal]  ┌────────┐
//!       ├────────────────────► Locked  │
//!       │                    └────┬────┘
//!       │ ◄── unlock ─────────────┘
//! ```
//!
//! # Platform Support
//!
//! | Feature            | Windows | macOS | Linux |
//! |--------------------|---------|-------|-------|
//! | Idle time polling  | ✅ (GetLastInputInfo) | ⚠️ graceful fallback | ⚠️ graceful fallback |
//! | Sleep/wake detect  | ✅ (sleep-gap method) | ✅ | ✅ |
//! | Screen lock detect | ⚠️ (covered by idle)  | ⚠️ | ⚠️ |
//! | Lid close detect   | ⚠️ (graceful skip)    | ⚠️ | ⚠️ |
//!
//! On platforms where OS idle time is not available, the scheduler is never
//! paused due to idle (always treated as active). Sleep/wake detection works
//! on all platforms through the sleep-gap method.
//!
//! # Thread Safety
//!
//! `ActivityMonitorService` wraps all mutable state in `Arc<Mutex<_>>`.
//! Duplicate monitor tasks are prevented by checking `task_handle` before
//! spawning — only one polling loop may run at any time.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::task::AbortHandle;

use crate::constants::{
    DEFAULT_IDLE_TIMEOUT_MINUTES, IDLE_POLL_INTERVAL_SECONDS, SLEEP_GAP_MULTIPLIER,
};
use crate::services::scheduler_service::SchedulerService;

// ── Monitor state ─────────────────────────────────────────────────────────────

/// The four states of the Active Usage state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum MonitorState {
    /// User is actively using the machine. Scheduler is running (or will run).
    Active,
    /// User has been idle beyond `DEFAULT_IDLE_TIMEOUT_MINUTES`. Scheduler is paused.
    Idle,
    /// System went to sleep. Scheduler is paused until the next wake.
    Sleeping,
    /// Momentary state upon waking. Re-evaluates activity before resuming.
    Wake,
    /// Screen is locked. Scheduler is paused until unlock.
    ///
    /// On platforms without an explicit lock signal this state is reached via
    /// the idle path (high idle time implies a locked screen).
    Locked,
}

impl std::fmt::Display for MonitorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active   => write!(f, "Active"),
            Self::Idle     => write!(f, "Idle"),
            Self::Sleeping => write!(f, "Sleeping"),
            Self::Wake     => write!(f, "Wake"),
            Self::Locked   => write!(f, "Locked"),
        }
    }
}

// ── Inner mutable state ───────────────────────────────────────────────────────

struct InnerState {
    /// Current monitor state.
    state: MonitorState,
    /// Handle for the running poll task. `None` when the monitor is stopped.
    /// Invariant: only one task may exist at a time.
    task_handle: Option<AbortHandle>,
}

// ── ActivityMonitorService ────────────────────────────────────────────────────

/// Active Usage Monitor.
///
/// `ActivityMonitorService` is `Clone` — it wraps all mutable state in an
/// `Arc<Mutex<_>>` so multiple handles to the same monitor can coexist safely.
///
/// # Usage
///
/// ```rust,ignore
/// let monitor = ActivityMonitorService::new();
/// monitor.start(scheduler.clone()).await;
/// // …
/// monitor.stop().await;
/// ```
#[derive(Clone)]
pub struct ActivityMonitorService {
    inner: Arc<Mutex<InnerState>>,
}

impl ActivityMonitorService {
    /// Creates a new monitor in the `Active` state with no running poll task.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState {
                state:       MonitorState::Active,
                task_handle: None,
            })),
        }
    }

    /// Returns the current monitor state without blocking for longer than a lock acquisition.
    pub fn state(&self) -> MonitorState {
        self.inner
            .lock()
            .map(|g| g.state.clone())
            .unwrap_or(MonitorState::Active)
    }

    /// Starts the monitoring loop.
    ///
    /// The loop polls the OS idle API every `IDLE_POLL_INTERVAL_SECONDS` and
    /// uses sleep-gap detection for system sleep/wake events.
    ///
    /// If the monitor is already running this call is a **no-op** (duplicate
    /// monitors are prevented — only one poll task may exist at a time).
    ///
    /// # Arguments
    ///
    /// * `scheduler` — The `SchedulerService` to pause/resume. It is cloned
    ///   into the spawned task; the original is not consumed.
    pub async fn start(&self, scheduler: SchedulerService) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("ActivityMonitor: failed to acquire lock in start(): {}", e);
                return;
            }
        };

        // Prevent duplicate monitors.
        if guard.task_handle.is_some() {
            log::warn!("ActivityMonitor: start() called while already running — ignoring.");
            return;
        }

        let handle = Self::spawn_poll_loop(Arc::clone(&self.inner), scheduler);
        guard.task_handle = Some(handle);

        log::info!("ActivityMonitor: Started (poll interval: {}s, idle timeout: {}min).",
            IDLE_POLL_INTERVAL_SECONDS,
            DEFAULT_IDLE_TIMEOUT_MINUTES
        );
    }

    /// Stops the monitoring loop and releases the poll task.
    ///
    /// The monitor state is reset to `Active` so a subsequent `start()` begins fresh.
    pub async fn stop(&self) {
        let mut guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => {
                log::error!("ActivityMonitor: failed to acquire lock in stop(): {}", e);
                return;
            }
        };

        if let Some(handle) = guard.task_handle.take() {
            handle.abort();
        }

        guard.state = MonitorState::Active;
        log::info!("ActivityMonitor: Stopped.");
    }

    // ── Private poll loop ─────────────────────────────────────────────────────

    fn spawn_poll_loop(
        inner: Arc<Mutex<InnerState>>,
        scheduler: SchedulerService,
    ) -> AbortHandle {
        let task = tokio::spawn(async move {
            let poll_interval   = Duration::from_secs(IDLE_POLL_INTERVAL_SECONDS);
            let idle_threshold  = Duration::from_secs(
                (DEFAULT_IDLE_TIMEOUT_MINUTES as u64).saturating_mul(60)
            );
            let sleep_gap_threshold = poll_interval
                .checked_mul(SLEEP_GAP_MULTIPLIER)
                .unwrap_or(poll_interval * 4);

            // Track time of the last tick to detect sleep gaps.
            let mut last_tick_at = Instant::now();

            // Track the state local to the task to avoid a lock per transition check.
            let mut local_state = MonitorState::Active;

            // Use a fixed interval — this also helps catch gap anomalies.
            let mut ticker = tokio::time::interval(poll_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ticker.tick().await; // skip the immediate tick at t=0

            loop {
                ticker.tick().await;

                let now        = Instant::now();
                let actual_gap = now.saturating_duration_since(last_tick_at);
                last_tick_at   = now;

                // ── Step 1: Sleep-gap detection ───────────────────────────────
                //
                // If the wall-clock gap between two consecutive poll ticks is
                // much larger than the expected interval, the system slept (our
                // task was suspended by the OS power manager) and has now woken.
                // We handle sleep AND wake in the same iteration — there is no
                // way to detect the sleep onset, only the wake.

                if actual_gap > sleep_gap_threshold {
                    log::info!(
                        "ActivityMonitor: System sleep detected (gap: {:.1}s > threshold: {:.1}s).",
                        actual_gap.as_secs_f64(),
                        sleep_gap_threshold.as_secs_f64()
                    );

                    // Transition through Sleeping.
                    if local_state != MonitorState::Sleeping {
                        if local_state == MonitorState::Active {
                            scheduler.pause().await;
                            log::info!("ActivityMonitor: Scheduler paused (sleep).");
                        }
                        set_state(&inner, MonitorState::Sleeping);
                        local_state = MonitorState::Sleeping;
                    }

                    log::info!("ActivityMonitor: System wake detected.");
                    
                    // Transition to Wake state momentarily
                    set_state(&inner, MonitorState::Wake);
                    local_state = MonitorState::Wake;

                    // Immediately evaluate the current OS idle duration
                    let idle_duration = get_idle_secs().map(Duration::from_secs);
                    let is_idle = idle_duration.map(|d| d >= idle_threshold).unwrap_or(false);

                    if !is_idle {
                        scheduler.resume().await;
                        log::info!("ActivityMonitor: Scheduler resumed (wake - user active).");
                        set_state(&inner, MonitorState::Active);
                        local_state = MonitorState::Active;
                    } else {
                        log::info!(
                            "ActivityMonitor: User idle upon wake (idle ≥ {}min). Scheduler remains paused.",
                            DEFAULT_IDLE_TIMEOUT_MINUTES
                        );
                        set_state(&inner, MonitorState::Idle);
                        local_state = MonitorState::Idle;
                    }

                    // Reset so the very next tick doesn't re-trigger sleep detection.
                    last_tick_at = Instant::now();
                    continue;
                }

                // ── Step 2: OS idle time detection ────────────────────────────
                //
                // `get_idle_secs()` returns the number of seconds since the OS
                // last observed keyboard or pointer input.  Returns `None` on
                // platforms where the API is not implemented — in that case we
                // treat the user as Active (never pause due to idle).

                let idle_duration = get_idle_secs().map(Duration::from_secs);

                let is_idle = idle_duration
                    .map(|d| d >= idle_threshold)
                    .unwrap_or(false);

                if idle_duration.is_none() && local_state == MonitorState::Active {
                    // Platform has no idle API — log once at trace level; do nothing.
                    log::trace!(
                        "ActivityMonitor: OS idle API not available on this platform. \
                         Scheduler will not pause for idle."
                    );
                }

                // ── Step 3: State transitions ─────────────────────────────────

                let target_state = if is_idle {
                    MonitorState::Idle
                } else {
                    MonitorState::Active
                };

                // Only act on changes to avoid hammering pause()/resume().
                if target_state == local_state {
                    continue;
                }

                match (&local_state, &target_state) {
                    (MonitorState::Active, MonitorState::Idle) => {
                        log::info!(
                            "ActivityMonitor: User idle detected (idle ≥ {}min). Pausing scheduler.",
                            DEFAULT_IDLE_TIMEOUT_MINUTES
                        );
                        scheduler.pause().await;
                        log::info!("ActivityMonitor: Scheduler paused (idle).");
                    }
                    (MonitorState::Idle, MonitorState::Active) => {
                        log::info!("ActivityMonitor: Activity detected. Resuming scheduler.");
                        scheduler.resume().await;
                        log::info!("ActivityMonitor: Scheduler resumed (activity).");
                    }
                    _ => {
                        // No direct scheduler action for other transitions.
                    }
                }

                set_state(&inner, target_state.clone());
                local_state = target_state;
            }
        });

        task.abort_handle()
    }
}

impl Default for ActivityMonitorService {
    fn default() -> Self {
        Self::new()
    }
}

// ── Shared state helper ───────────────────────────────────────────────────────

/// Updates the shared `MonitorState` without holding the lock across async awaits.
fn set_state(inner: &Arc<Mutex<InnerState>>, new_state: MonitorState) {
    match inner.lock() {
        Ok(mut g) => g.state = new_state,
        Err(e) => {
            log::error!("ActivityMonitor: failed to update shared state: {}", e);
        }
    }
}

// ── Platform-specific idle time ───────────────────────────────────────────────
//
// Each platform returns the number of **seconds** since the last user input
// event (keyboard or mouse). Returns `None` if the platform does not implement
// this capability — the caller treats `None` as "always active".

/// Returns seconds since last user input, or `None` if unsupported.
#[cfg(target_os = "windows")]
fn get_idle_secs() -> Option<u64> {
    use std::mem;

    // SAFETY: We call Win32 functions with correctly sized structs. The
    // `LASTINPUTINFO.cbSize` field is explicitly set before calling
    // `GetLastInputInfo`. No memory aliasing or unsound access occurs.
    unsafe {
        use windows_sys::Win32::System::SystemInformation::GetTickCount;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

        let mut lii = LASTINPUTINFO {
            cbSize: mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        if GetLastInputInfo(&mut lii) != 0 {
            // Both values are 32-bit tick counts (milliseconds since boot).
            // Wrapping subtraction handles the ~49-day overflow correctly.
            let idle_ms = GetTickCount().wrapping_sub(lii.dwTime) as u64;
            Some(idle_ms / 1000)
        } else {
            log::warn!("ActivityMonitor: GetLastInputInfo failed — treating as active.");
            None
        }
    }
}

/// Returns `None` — idle detection is not implemented on macOS yet.
///
/// The sleep-gap method still detects system sleep/wake on this platform.
/// A future phase can add CoreGraphics/IOKit bindings for full idle support.
#[cfg(target_os = "macos")]
fn get_idle_secs() -> Option<u64> {
    None
}

/// Returns `None` — idle detection requires X11/Wayland extensions on Linux.
///
/// The sleep-gap method still detects system sleep/wake on this platform.
/// A future phase can add the `xscreensaver` or `logind` D-Bus interface.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_idle_secs() -> Option<u64> {
    None
}
