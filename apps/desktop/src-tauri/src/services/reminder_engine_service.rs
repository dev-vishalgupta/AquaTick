//! Reminder Engine — orchestrates the complete reminder lifecycle.
//!
//! # Scope
//!
//! `ReminderEngineService` is the **single owner** of:
//! - Reminder lifecycle (pending ? triggered ? completed / timed_out / snoozed)
//! - Hydration Session creation and state transitions
//! - Session timeout management
//! - Snooze workflow
//! - Scheduler coordination (start / stop / reset after lifecycle ends)
//!
//! This module has **zero knowledge** of:
//! - UI / character / animations
//! - Sound
//! - Frontend / React / Tauri events
//! - Statistics (computation deferred to a later phase)
//!
//! # Architecture position
//!
//! ```text
//!   SchedulerService         (owns: countdown timer only)
//!          ¦  on_reminder_due callback
//!          ?
//!   ReminderEngineService    (owns: session lifecycle, timeout, snooze)
//!          ¦  SessionService calls
//!          ?
//!   SessionRepository / DB
//! ```
//!
//! # Engine State Machine
//!
//! ```text
//!   +------+  handle_reminder_due  +---------------+
//!   ¦ Idle ¦ -------------------? ¦ SessionActive ¦
//!   +------+                       +---------------+
//!                                          ¦ complete_session()
//!                                          ¦   ? Completed  ? Idle (scheduler reset)
//!                                          ¦ timeout_session()
//!                                          ¦   ? TimedOut   ? Idle (scheduler reset)
//!                                          ¦ snooze_session()
//!                                          ¦   ? Snoozed
//!                                          ?
//!                                    +---------+
//!                                    ¦ Snoozed ¦
//!                                    +---------+
//!                                         ¦ snooze timer fires ? SessionActive (re-triggered)
//! ```
//!
//! # Thread Safety
//!
//! All mutable state lives in `Arc<Mutex<InnerEngineState>>`. No `.unwrap()` or
//! `panic!()` appears in this module. Lock failures are logged and treated as
//! no-ops — the engine remains in its last known state.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::{Pool, Sqlite};
use tokio::task::AbortHandle;

use crate::constants::DEFAULT_SESSION_TIMEOUT_MINUTES;
use crate::errors::AppResult;
use crate::models::AppSettings;
use crate::services::{scheduler_service::OnReminderDue, SchedulerService, SessionService};

// -- Engine state --------------------------------------------------------------

/// The three states of the Reminder Engine.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineState {
    /// No active session. Scheduler is running toward the next reminder.
    Idle,
    /// A session is in `triggered` status. Timeout timer is running.
    SessionActive,
    /// Session is in `snoozed` status. Snooze timer is running.
    Snoozed,
}

impl std::fmt::Display for EngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle          => write!(f, "Idle"),
            Self::SessionActive => write!(f, "SessionActive"),
            Self::Snoozed       => write!(f, "Snoozed"),
        }
    }
}

// -- Inner mutable state -------------------------------------------------------

struct InnerEngineState {
    /// Current engine state.
    engine_state: EngineState,

    /// The currently active session ID.
    ///
    /// `Some(id)` in `SessionActive` and `Snoozed` states.
    /// `None` in `Idle`.
    ///
    /// Invariant S-1: only one active session may exist.
    active_session_id: Option<i64>,

    /// AbortHandle for the session timeout task.
    ///
    /// Invariant T-3: only one timeout timer may exist at any time.
    timeout_handle: Option<AbortHandle>,

    /// AbortHandle for the snooze timer task.
    ///
    /// Invariant T-4: only one snooze timer may exist at any time.
    snooze_handle: Option<AbortHandle>,

    /// Tauri AppHandle to emit events.
    app_handle: Option<tauri::AppHandle>,
}

impl InnerEngineState {
    fn new() -> Self {
        Self {
            engine_state:      EngineState::Idle,
            active_session_id: None,
            timeout_handle:    None,
            snooze_handle:     None,
            app_handle:        None,
        }
    }

    /// Cancels the active timeout timer if one exists. Idempotent.
    fn cancel_timeout(&mut self) {
        if let Some(h) = self.timeout_handle.take() {
            h.abort();
        }
    }

    /// Cancels the active snooze timer if one exists. Idempotent.
    fn cancel_snooze(&mut self) {
        if let Some(h) = self.snooze_handle.take() {
            h.abort();
        }
    }
}

// -- ReminderEngineService -----------------------------------------------------

/// Reminder Engine — single orchestrator of the reminder lifecycle.
///
/// `ReminderEngineService` is `Clone` — it wraps all mutable state in an
/// `Arc<Mutex<_>>` so multiple handles can coexist safely.
#[derive(Clone)]
pub struct ReminderEngineService {
    inner: Arc<Mutex<InnerEngineState>>,
}

impl ReminderEngineService {
    /// Creates a new engine in the `Idle` state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerEngineState::new())),
        }
    }

    /// Sets the tauri AppHandle for emitting IPC events to the frontend.
    pub fn set_app_handle(&self, app_handle: tauri::AppHandle) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.app_handle = Some(app_handle);
        }
    }

    /// Returns the current engine state.
    pub fn state(&self) -> EngineState {
        self.inner
            .lock()
            .map(|g| g.engine_state.clone())
            .unwrap_or(EngineState::Idle)
    }

    /// Returns the active session ID, if any.
    pub fn active_session_id(&self) -> Option<i64> {
        self.inner
            .lock()
            .map(|g| g.active_session_id)
            .unwrap_or(None)
    }

    // -- Lifecycle methods -----------------------------------------------------

    /// Handles a reminder-due notification from the `SchedulerService`.
    ///
    /// Creates a pending session, marks it triggered, and starts the session
    /// timeout timer. If a session is already active, the notification is
    /// discarded (guards against concurrent triggers).
    ///
    /// Called from within the `OnReminderDue` callback — must not block.
    pub fn handle_reminder_due(
        &self,
        pool: Pool<Sqlite>,
        settings: AppSettings,
        scheduler: SchedulerService,
    ) {
        // Guard: only one active session permitted.
        {
            let guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => {
                    log::error!("ReminderEngine: lock failed in handle_reminder_due(): {}", e);
                    return;
                }
            };
            if guard.engine_state != EngineState::Idle {
                log::warn!(
                    "ReminderEngine: handle_reminder_due() called while in state '{}' — discarding.",
                    guard.engine_state
                );
                return;
            }
        }

        log::info!("ReminderEngine: Reminder due. Starting session creation.");

        let engine = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.create_and_trigger_session(&pool, &settings, scheduler).await {
                log::error!("ReminderEngine: failed to create/trigger session: {}", e);
            }
        });
    }

    /// Resolves the active session as `completed`.
    ///
    /// Valid from `SessionActive` only. Returns an error for invalid transitions
    /// or if no session is active.
    pub async fn complete_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        let active_id = self.validate_active_session(session_id, "complete_session")?;

        log::info!("ReminderEngine: Completing session ID {}.", active_id);

        // Cancel timeout timer before DB write.
        self.cancel_timeout_timer();

        // Mark session completed in the DB.
        SessionService::complete(pool, active_id).await?;

        // Clear engine state.
        self.clear_session_state(EngineState::Idle);

        log::info!("ReminderEngine: Session {} completed.", active_id);

        // Emit session:completed event to frontend
        {
            use tauri::Emitter;
            if let Ok(guard) = self.inner.lock() {
                if let Some(app) = &guard.app_handle {
                    #[derive(serde::Serialize, Clone)]
                    struct CompletedPayload {
                        #[serde(rename = "sessionId")]
                        session_id: String,
                    }
                    let payload = CompletedPayload {
                        session_id: active_id.to_string(),
                    };
                    app.emit("session:completed", payload).ok();
                }
            }
        }

        // Restart the scheduler for the next reminder cycle.
        scheduler.reset().await;
        log::info!("ReminderEngine: Scheduler reset after session completion.");

        Ok(())
    }

    /// Snoozes the active session.
    ///
    /// Valid from `SessionActive` only. Increments the snooze count, transitions
    /// the session to `snoozed`, and schedules the snooze timer. When the snooze
    /// timer fires, the session is re-triggered.
    pub async fn snooze_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        delay_minutes: i64,
        settings: &AppSettings,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        let active_id = self.validate_active_session(session_id, "snooze_session")?;

        log::info!("ReminderEngine: Snoozing session ID {} for {} minutes.", active_id, delay_minutes);

        // Cancel running timeout.
        self.cancel_timeout_timer();

        // Transition session to snoozed in DB.
        SessionService::snooze(pool, active_id).await?;

        // Transition engine state to Snoozed.
        {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => {
                    log::error!("ReminderEngine: lock failed transitioning to Snoozed: {}", e);
                    return Err(crate::errors::AppError::Internal(e.to_string()));
                }
            };
            guard.engine_state = EngineState::Snoozed;
        }

        log::info!("ReminderEngine: Session {} snoozed.", active_id);

        // Emit session:snoozed event to frontend
        {
            use tauri::Emitter;
            if let Ok(guard) = self.inner.lock() {
                if let Some(app) = &guard.app_handle {
                    #[derive(serde::Serialize, Clone)]
                    struct SnoozedPayload {
                        #[serde(rename = "sessionId")]
                        session_id: String,
                        #[serde(rename = "durationMin")]
                        duration_min: i64,
                    }
                    let payload = SnoozedPayload {
                        session_id: active_id.to_string(),
                        duration_min: delay_minutes,
                    };
                    app.emit("session:snoozed", payload).ok();
                }
            }
        }

        // Spawn the snooze timer.
        let snooze_handle = self.spawn_snooze_timer(
            pool.clone(),
            active_id,
            delay_minutes,
            settings.clone(),
            scheduler.clone(),
        );

        // Store the snooze handle.
        match self.inner.lock() {
            Ok(mut g) => {
                g.cancel_snooze();
                g.snooze_handle = Some(snooze_handle);
            }
            Err(e) => {
                log::error!("ReminderEngine: failed to store snooze handle: {}", e);
            }
        }

        Ok(())
    }

    /// Transitions the active session to `timed_out` and resets the scheduler.
    ///
    /// Called by the internal timeout timer or during shutdown.
    pub async fn timeout_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        // A timeout can fire from SessionActive; allow it even if state was cleared.
        let active_id = {
            let guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => {
                    return Err(crate::errors::AppError::Internal(e.to_string()));
                }
            };
            guard.active_session_id
        };

        let active_id = match active_id {
            Some(id) if id == session_id => id,
            Some(other) => {
                log::warn!(
                    "ReminderEngine: timeout_session() called for ID {} but active is {:?} — ignoring.",
                    session_id, other
                );
                return Ok(());
            }
            None => {
                log::warn!(
                    "ReminderEngine: timeout_session() called but no active session — ignoring."
                );
                return Ok(());
            }
        };

        log::info!("ReminderEngine: Session {} timed out.", active_id);

        // Cancel any remaining timers.
        self.cancel_timeout_timer();
        self.cancel_snooze_timer();

        // Mark session timed_out in DB.
        SessionService::mark_timed_out(pool, active_id).await?;

        // Clear engine state.
        self.clear_session_state(EngineState::Idle);

        log::info!("ReminderEngine: Session {} transitioned to timed_out.", active_id);

        // Emit session:timedOut event to frontend
        {
            use tauri::Emitter;
            if let Ok(guard) = self.inner.lock() {
                if let Some(app) = &guard.app_handle {
                    #[derive(serde::Serialize, Clone)]
                    struct TimedOutPayload {
                        #[serde(rename = "sessionId")]
                        session_id: String,
                    }
                    let payload = TimedOutPayload {
                        session_id: active_id.to_string(),
                    };
                    app.emit("session:timedOut", payload).ok();
                }
            }
        }

        // Restart the scheduler.
        scheduler.reset().await;
        log::info!("ReminderEngine: Scheduler reset after session timeout.");

        Ok(())
    }

    /// Cancels the active session during application shutdown.
    ///
    /// Forces any `triggered` or `snoozed` session to `timed_out` without
    /// restarting the scheduler (application is shutting down).
    pub async fn cancel_active_session(&self, pool: &Pool<Sqlite>) {
        let active_id = {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => { log::error!("ReminderEngine: lock failed in cancel_active_session(): {}", e); return; }
            };
            let id = guard.active_session_id;
            guard.cancel_timeout();
            guard.cancel_snooze();
            id
        };

        if let Some(id) = active_id {
            log::info!("ReminderEngine: Cancelling active session {} on shutdown.", id);
            if let Err(e) = SessionService::mark_timed_out(pool, id).await {
                log::error!("ReminderEngine: failed to cancel session {} on shutdown: {}", id, e);
            }
            self.clear_session_state(EngineState::Idle);
        }
    }

    /// Recovers stale sessions from a previous crash or unexpected shutdown.
    ///
    /// Any sessions left in `triggered` status longer than `DEFAULT_SESSION_TIMEOUT_MINUTES`
    /// are resolved as `timed_out`. Called once at application startup before the
    /// first scheduler start.
    pub async fn recover_stale_sessions(&self, pool: &Pool<Sqlite>) {
        log::info!("ReminderEngine: Starting stale session recovery.");

        let now_ms = chrono::Utc::now().timestamp_millis();
        match SessionService::recover_stale(pool, now_ms, DEFAULT_SESSION_TIMEOUT_MINUTES).await {
            Ok(count) if count > 0 => {
                log::info!("ReminderEngine: Recovered {} stale session(s).", count);
            }
            Ok(_) => {
                log::info!("ReminderEngine: No stale sessions found.");
            }
            Err(e) => {
                log::error!("ReminderEngine: stale session recovery failed: {}", e);
            }
        }

        log::info!("ReminderEngine: Stale session recovery finished.");
    }

    // -- Private helpers -------------------------------------------------------

    /// Creates a pending session, marks it triggered, and starts the timeout timer.
    async fn create_and_trigger_session(
        &self,
        pool: &Pool<Sqlite>,
        settings: &AppSettings,
        scheduler: SchedulerService,
    ) -> AppResult<()> {
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Create pending session.
        let session = SessionService::create_pending(
            pool,
            now_ms,
            settings.reminder_interval_minutes,
            settings.character_id.clone(),
            None, // sound_id: deferred to Reminder Engine Phase 4D (sound system)
        )
        .await?;

        log::info!("ReminderEngine: Created pending session ID {}.", session.id);

        // Transition pending ? triggered.
        SessionService::mark_triggered(pool, session.id).await?;
        log::info!("ReminderEngine: Session {} triggered.", session.id);

        let session_id = session.id;

        // Emit session:triggered event to frontend
        {
            use tauri::Emitter;
            if let Ok(guard) = self.inner.lock() {
                if let Some(app) = &guard.app_handle {
                    #[derive(serde::Serialize, Clone)]
                    struct TriggeredPayload {
                        #[serde(rename = "sessionId")]
                        session_id: String,
                        #[serde(rename = "dueAt")]
                        due_at: String,
                    }
                    let payload = TriggeredPayload {
                        session_id: session_id.to_string(),
                        due_at: chrono::Utc::now().to_rfc3339(),
                    };
                    app.emit("session:triggered", payload).ok();
                }
            }
        }

        // Store active session and transition engine state.
        {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
            };
            guard.cancel_timeout(); // safety: should be None, but be defensive
            guard.active_session_id = Some(session_id);
            guard.engine_state      = EngineState::SessionActive;
        }

        // Start the session timeout timer.
        let timeout_handle = self.spawn_timeout_timer(
            pool.clone(),
            session_id,
            DEFAULT_SESSION_TIMEOUT_MINUTES,
            scheduler.clone(),
        );

        match self.inner.lock() {
            Ok(mut g) => {
                g.cancel_timeout();
                g.timeout_handle = Some(timeout_handle);
            }
            Err(e) => {
                log::error!("ReminderEngine: failed to store timeout handle: {}", e);
            }
        }

        log::info!("ReminderEngine: Session {} active. Timeout timer started ({} min).",
            session_id, DEFAULT_SESSION_TIMEOUT_MINUTES);

        Ok(())
    }

    /// Spawns an async task that fires `timeout_session` after the configured timeout.
    fn spawn_timeout_timer(
        &self,
        pool:            Pool<Sqlite>,
        session_id:      i64,
        timeout_minutes: i64,
        scheduler:       SchedulerService,
    ) -> AbortHandle {
        let engine = self.clone();
        let duration = Duration::from_secs((timeout_minutes as u64).saturating_mul(60));

        let task = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            log::info!("ReminderEngine: Timeout timer fired for session {}.", session_id);

            if let Err(e) = engine.timeout_session(&pool, session_id, &scheduler).await {
                log::error!("ReminderEngine: timeout_session failed for {}: {}", session_id, e);
            }
        });

        task.abort_handle()
    }

    /// Spawns an async task that re-triggers the session after the snooze delay.
    fn spawn_snooze_timer(
        &self,
        pool:          Pool<Sqlite>,
        session_id:    i64,
        delay_minutes: i64,
        settings:      AppSettings,
        scheduler:     SchedulerService,
    ) -> AbortHandle {
        let engine   = self.clone();
        let duration = Duration::from_secs((delay_minutes as u64).saturating_mul(60));

        let task = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            log::info!("ReminderEngine: Snooze timer fired for session {}.", session_id);

            if let Err(e) = engine.handle_snooze_expired(&pool, session_id, &settings, &scheduler).await {
                log::error!("ReminderEngine: snooze re-trigger failed for {}: {}", session_id, e);
            }
        });

        task.abort_handle()
    }

    /// Re-triggers a snoozed session: transitions `snoozed ? triggered` and restarts
    /// the timeout timer. Same session ID, snooze_count already incremented.
    async fn handle_snooze_expired(
        &self,
        pool:      &Pool<Sqlite>,
        session_id: i64,
        settings:  &AppSettings,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        // Verify the engine is still in Snoozed state for this session.
        {
            let guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
            };

            if guard.engine_state != EngineState::Snoozed {
                log::warn!(
                    "ReminderEngine: snooze expired but engine state is '{}' — ignoring.",
                    guard.engine_state
                );
                return Ok(());
            }

            if guard.active_session_id != Some(session_id) {
                log::warn!(
                    "ReminderEngine: snooze expired for session {} but active is {:?} — ignoring.",
                    session_id, guard.active_session_id
                );
                return Ok(());
            }
        }

        // Transition snoozed ? triggered in the DB.
        SessionService::mark_triggered(pool, session_id).await?;
        log::info!("ReminderEngine: Session {} re-triggered after snooze.", session_id);

        // Emit session:triggered event to frontend
        {
            use tauri::Emitter;
            if let Ok(guard) = self.inner.lock() {
                if let Some(app) = &guard.app_handle {
                    #[derive(serde::Serialize, Clone)]
                    struct TriggeredPayload {
                        #[serde(rename = "sessionId")]
                        session_id: String,
                        #[serde(rename = "dueAt")]
                        due_at: String,
                    }
                    let payload = TriggeredPayload {
                        session_id: session_id.to_string(),
                        due_at: chrono::Utc::now().to_rfc3339(),
                    };
                    app.emit("session:triggered", payload).ok();
                }
            }
        }

        // Transition engine state back to SessionActive.
        {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
            };
            guard.cancel_snooze();
            guard.engine_state = EngineState::SessionActive;
        }

        // Start a fresh timeout timer.
        let timeout_handle = self.spawn_timeout_timer(
            pool.clone(),
            session_id,
            DEFAULT_SESSION_TIMEOUT_MINUTES,
            scheduler.clone(),
        );

        match self.inner.lock() {
            Ok(mut g) => {
                g.cancel_timeout();
                g.timeout_handle = Some(timeout_handle);
            }
            Err(e) => {
                log::error!("ReminderEngine: failed to store timeout handle after snooze: {}", e);
            }
        }

        log::info!("ReminderEngine: Session {} active again (post-snooze). Timeout timer started.",
            session_id);

        let _ = settings; // reserved for future use (e.g. re-reading interval on resume)
        let _ = scheduler;
        Ok(())
    }

    /// Validates that `session_id` matches the currently active session and
    /// that the engine is in `SessionActive`. Returns the confirmed active ID.
    fn validate_active_session(&self, session_id: i64, caller: &str) -> AppResult<i64> {
        let guard = match self.inner.lock() {
            Ok(g)  => g,
            Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
        };

        if guard.engine_state != EngineState::SessionActive {
            return Err(crate::errors::AppError::Validation(format!(
                "ReminderEngine: {}() called while in state '{}' — expected SessionActive.",
                caller, guard.engine_state
            )));
        }

        match guard.active_session_id {
            Some(id) if id == session_id => Ok(id),
            Some(other) => Err(crate::errors::AppError::Validation(format!(
                "ReminderEngine: {}() session_id {} does not match active session {}.",
                caller, session_id, other
            ))),
            None => Err(crate::errors::AppError::Internal(format!(
                "ReminderEngine: {}() — no active session ID despite SessionActive state.",
                caller
            ))),
        }
    }

    /// Clears all active session state and transitions to `new_state`.
    fn clear_session_state(&self, new_state: EngineState) {
        match self.inner.lock() {
            Ok(mut g) => {
                g.cancel_timeout();
                g.cancel_snooze();
                g.active_session_id = None;
                g.engine_state      = new_state;
            }
            Err(e) => {
                log::error!("ReminderEngine: failed to clear session state: {}", e);
            }
        }
    }

    /// Cancels the timeout timer (does not modify other state).
    fn cancel_timeout_timer(&self) {
        match self.inner.lock() {
            Ok(mut g) => g.cancel_timeout(),
            Err(e)    => log::error!("ReminderEngine: failed to cancel timeout: {}", e),
        }
    }

    /// Cancels the snooze timer (does not modify other state).
    fn cancel_snooze_timer(&self) {
        match self.inner.lock() {
            Ok(mut g) => g.cancel_snooze(),
            Err(e)    => log::error!("ReminderEngine: failed to cancel snooze: {}", e),
        }
    }
}

impl Default for ReminderEngineService {
    fn default() -> Self { Self::new() }
}

// -- Engine callback builder ---------------------------------------------------

/// Builds the `OnReminderDue` callback that wires the scheduler to the engine.
///
/// Called once during application startup to construct the closure passed to
/// `SchedulerService::start()`. The closure captures a clone of `ReminderEngineService`,
/// the db pool, and the current settings snapshot.
pub fn build_reminder_due_callback(
    engine:    ReminderEngineService,
    pool:      Pool<Sqlite>,
    settings:  AppSettings,
    scheduler: SchedulerService,
) -> OnReminderDue {
    Arc::new(move || {
        engine.handle_reminder_due(pool.clone(), settings.clone(), scheduler.clone());
    })
}
