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
//!   Idle -> SessionActive -> Resolving -> Idle (terminal outcome)
//!                              |          -> Snoozed (snooze outcome)
//!                              +-> SessionActive on recoverable persistence failure
//!   Snoozed -> SessionActive when the snooze timer re-triggers the same session
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
use crate::models::{AppSettings, SessionStatus};
use crate::repositories::SessionRepository;
use crate::services::{scheduler_service::OnReminderDue, SchedulerService, SessionService, StatisticsService};

// -- Engine state --------------------------------------------------------------

/// The four states of the Reminder Engine.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineState {
    /// No active session. Scheduler is running toward the next reminder.
    Idle,
    /// A session is in `triggered` status. Timeout timer is running.
    SessionActive,
    /// An outcome has been reserved and its persistence is in progress.
    Resolving,
    /// Session is in `snoozed` status. Snooze timer is running.
    Snoozed,
}

impl std::fmt::Display for EngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle          => write!(f, "Idle"),
            Self::SessionActive => write!(f, "SessionActive"),
            Self::Resolving     => write!(f, "Resolving"),
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
    /// `Some(id)` in `SessionActive`, `Resolving`, and `Snoozed` states.
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
    /// If engine state is Idle, creates a pending session and triggers it.
    /// If engine state is Snoozed, re-triggers the existing active session.
    /// If engine state is SessionActive, the notification is discarded.
    ///
    /// Called from within the `OnReminderDue` callback — must not block.
    pub fn handle_reminder_due(
        &self,
        pool: Pool<Sqlite>,
        scheduler: SchedulerService,
    ) {
        let (state, active_id) = {
            let guard = match self.inner.lock() {
                Ok(g)  => (g.engine_state.clone(), g.active_session_id),
                Err(e) => {
                    log::error!("ReminderEngine: lock failed in handle_reminder_due(): {}", e);
                    return;
                }
            };
            (guard.0, guard.1)
        };

        if state == EngineState::SessionActive {
            log::warn!("ReminderEngine: handle_reminder_due() called while in SessionActive state — discarding.");
            return;
        }

        log::info!("ReminderEngine: Reminder due. Engine state: '{}', active_session: {:?}", state, active_id);

        let engine = self.clone();
        tokio::spawn(async move {
            if state == EngineState::Snoozed {
                if let Some(id) = active_id {
                    if let Err(e) = engine.retrigger_snoozed_session(&pool, id, &scheduler).await {
                        log::error!("ReminderEngine: failed to re-trigger snoozed session: {}", e);
                    }
                }
            } else {
                if let Err(e) = engine.create_and_trigger_session(&pool, scheduler).await {
                    log::error!("ReminderEngine: failed to create/trigger session: {}", e);
                }
            }
        });
    }

    /// Resolves the active session as `completed`.
    pub async fn complete_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        active_ms_today: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        self.resolve_session(
            pool,
            session_id,
            SessionStatus::Completed,
            active_ms_today,
            None,
            scheduler,
        )
        .await
    }

    /// Snoozes the active session.
    pub async fn snooze_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        delay_minutes: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        self.resolve_session(
            pool,
            session_id,
            SessionStatus::Snoozed,
            -1, // Snooze is not terminal; stats ignored
            Some(delay_minutes),
            scheduler,
        )
        .await
    }

    /// Transitions the active session to `timed_out` (engine timeout decision).
    pub async fn timeout_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        active_ms_today: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        self.resolve_session(
            pool,
            session_id,
            SessionStatus::TimedOut,
            active_ms_today,
            None,
            scheduler,
        )
        .await
    }

    /// Resolves the active session as `ignored` (explicit user dismiss action).
    pub async fn ignore_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        active_ms_today: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        self.resolve_session(
            pool,
            session_id,
            SessionStatus::Ignored,
            active_ms_today,
            None,
            scheduler,
        )
        .await
    }

    /// Reserves a session outcome before spawning independently-owned persistence work.
    ///
    /// The spawned task deliberately outlives this caller. In particular, the timeout
    /// task may abort itself when the accepted resolution cancels its timer; that must
    /// not cancel the transaction that records the timeout.
    async fn resolve_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        target_status: SessionStatus,
        active_ms_today: i64,
        snooze_delay_minutes: Option<i64>,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        if target_status == SessionStatus::Snoozed {
            validate_snooze_delay(snooze_delay_minutes.ok_or_else(|| {
                crate::errors::AppError::Validation("Snooze duration is required".into())
            })?)?;
        }

        let (active_id, previous_state, timeout_handle) = {
            let mut guard = self.inner.lock().map_err(|e| {
                crate::errors::AppError::Internal(e.to_string())
            })?;

            if guard.engine_state == EngineState::Idle {
                log::warn!("ReminderEngine: resolution requested while Idle; ignoring.");
                return Ok(());
            }

            if guard.engine_state == EngineState::Resolving {
                log::warn!("ReminderEngine: a resolution is already in progress; ignoring duplicate request.");
                return Ok(());
            }

            if guard.engine_state == EngineState::Snoozed
                && target_status == SessionStatus::Snoozed
            {
                log::warn!("ReminderEngine: session is already Snoozed; ignoring duplicate snooze.");
                return Ok(());
            }

            let active_id = match guard.active_session_id {
                Some(id) if id == session_id => id,
                Some(other) => {
                    log::warn!(
                        "ReminderEngine: resolution session {} does not match active session {}; ignoring.",
                        session_id, other
                    );
                    return Ok(());
                }
                None => {
                    return Err(crate::errors::AppError::Internal(
                        "ReminderEngine: active session ID missing in a non-Idle state.".into()
                    ));
                }
            };

            let previous_state = guard.engine_state.clone();
            guard.engine_state = EngineState::Resolving;
            // Taking the handle reserves/cancels timeout protection only after the
            // independent worker has been created below.
            let timeout_handle = guard.timeout_handle.take();
            (active_id, previous_state, timeout_handle)
        };

        let engine = self.clone();
        let pool = pool.clone();
        let scheduler = scheduler.clone();
        let previous_state_for_worker = previous_state.clone();
        let worker = tokio::spawn(async move {
            if let Some(handle) = timeout_handle {
                handle.abort();
            }

            let result = engine.resolve_session_operation(
                &pool,
                active_id,
                target_status,
                active_ms_today,
                snooze_delay_minutes,
                &scheduler,
            ).await;

            if result.is_err() {
                engine.restore_after_resolution_failure(
                    pool.clone(),
                    active_id,
                    previous_state_for_worker,
                    scheduler.clone(),
                );
            }

            result
        });

        match worker.await {
            Ok(result) => result,
            Err(error) => {
                self.restore_after_resolution_failure(
                    pool.clone(),
                    active_id,
                    previous_state,
                    scheduler.clone(),
                );
                Err(crate::errors::AppError::Internal(format!(
                    "Reminder Engine resolution task failed: {}", error
                )))
            }
        }
    }

    /// Performs the database transaction and post-commit handoff for an accepted resolution.
    async fn resolve_session_operation(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        target_status: SessionStatus,
        active_ms_today: i64,
        snooze_delay_minutes: Option<i64>,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        log::info!(
            "ReminderEngine: Persisting resolution for session {} to '{}'.",
            session_id, target_status
        );

        // Load settings before opening the transaction. A settings failure must not
        // be silently replaced with defaults or leave an open transaction.
        let terminal = target_status == SessionStatus::Completed
            || target_status == SessionStatus::TimedOut
            || target_status == SessionStatus::Ignored;
        let settings = if terminal {
            Some(crate::services::SettingsService::load_all(pool).await?)
        } else {
            None
        };

        let mut tx = pool.begin().await?;
        let session = SessionRepository::get_by_id(&mut *tx, session_id)
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(
                format!("Session {} not found", session_id)
            ))?;

        let current_status = std::str::FromStr::from_str(&session.status)?;
        SessionService::validate_transition(&current_status, &target_status)?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        if target_status == SessionStatus::Snoozed {
            SessionRepository::update_status(
                &mut *tx, session_id, target_status.clone(), None, now_ms
            ).await?;
            SessionRepository::increment_snooze(&mut *tx, session_id, now_ms).await?;
        } else {
            SessionRepository::update_status(
                &mut *tx, session_id, target_status.clone(), Some(now_ms), now_ms
            ).await?;
        }

        if let Some(settings) = settings {
            let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
            StatisticsService::recompute_for_date(
                &mut *tx,
                &today_str,
                active_ms_today,
                settings.reminder_interval_minutes,
            ).await?;
        }

        tx.commit().await?;
        log::info!(
            "ReminderEngine: Session {} committed with status '{}'.",
            session_id, target_status
        );

        // Publish the committed state before scheduler handoff so an immediate due
        // callback cannot observe Resolving and create a duplicate session.
        {
            let mut guard = self.inner.lock().map_err(|e| {
                crate::errors::AppError::Internal(e.to_string())
            })?;
            if guard.active_session_id != Some(session_id)
                || guard.engine_state != EngineState::Resolving
            {
                return Err(crate::errors::AppError::Internal(
                    "Reminder Engine state changed during committed resolution".into()
                ));
            }
            if target_status == SessionStatus::Snoozed {
                guard.engine_state = EngineState::Snoozed;
            } else {
                guard.engine_state = EngineState::Idle;
                guard.active_session_id = None;
            }
        }

        scheduler.report_resolution(&target_status, snooze_delay_minutes, pool).await;
        self.emit_outcome_events(session_id, &target_status, snooze_delay_minutes);
        Ok(())
    }

    /// Restores a recoverable engine state and timeout protection after persistence failure.
    fn restore_after_resolution_failure(
        &self,
        pool: Pool<Sqlite>,
        session_id: i64,
        previous_state: EngineState,
        scheduler: SchedulerService,
    ) {
        let should_restore_timeout = {
            let mut guard = match self.inner.lock() {
                Ok(guard) => guard,
                Err(error) => {
                    log::error!("ReminderEngine: failed to recover resolution state: {}", error);
                    return;
                }
            };

            if guard.active_session_id != Some(session_id)
                || guard.engine_state != EngineState::Resolving
            {
                return;
            }

            guard.engine_state = previous_state.clone();
            previous_state == EngineState::SessionActive
        };

        if should_restore_timeout {
            let timeout_handle = self.spawn_timeout_timer(
                pool,
                session_id,
                DEFAULT_SESSION_TIMEOUT_MINUTES,
                scheduler,
            );
            if let Ok(mut guard) = self.inner.lock() {
                if guard.active_session_id == Some(session_id)
                    && guard.engine_state == EngineState::SessionActive
                {
                    guard.cancel_timeout();
                    guard.timeout_handle = Some(timeout_handle);
                } else {
                    timeout_handle.abort();
                }
            } else {
                timeout_handle.abort();
            }
        }
    }

    /// Cancels the active session during application shutdown.
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
            let mut conn = match pool.acquire().await {
                Ok(c) => c,
                Err(e) => { log::error!("ReminderEngine: failed to acquire connection: {}", e); return; }
            };
            
            if let Err(e) = SessionRepository::update_status(
                &mut conn,
                id,
                SessionStatus::TimedOut,
                Some(chrono::Utc::now().timestamp_millis()),
                chrono::Utc::now().timestamp_millis()
            ).await {
                log::error!("ReminderEngine: failed to cancel session {} on shutdown: {}", id, e);
            }

            // Emit presentation dismissed
            {
                use tauri::Emitter;
                if let Ok(guard) = self.inner.lock() {
                    if let Some(app) = &guard.app_handle {
                        app.emit("reminder:dismissed", ()).ok();
                    }
                }
            }

            self.clear_session_state(EngineState::Idle);
        }
    }

    /// Recovers stale sessions from a previous crash or unexpected shutdown.
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

    /// Re-triggers a snoozed session.
    async fn retrigger_snoozed_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: i64,
        scheduler: &SchedulerService,
    ) -> AppResult<()> {
        log::info!("ReminderEngine: Re-triggering snoozed session ID {}.", session_id);

        let mut conn = pool.acquire().await?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        SessionRepository::update_triggered(&mut conn, session_id, SessionStatus::Triggered, now_ms, now_ms).await?;

        // Transition engine state to SessionActive.
        {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
            };
            guard.engine_state = EngineState::SessionActive;
        }

        // Emit trigger events
        self.emit_trigger_events(session_id);

        // Start a fresh timeout timer.
        let timeout_handle = self.spawn_timeout_timer(
            pool.clone(),
            session_id,
            DEFAULT_SESSION_TIMEOUT_MINUTES,
            scheduler.clone(),
        );

        if let Ok(mut guard) = self.inner.lock() {
            guard.cancel_timeout();
            guard.timeout_handle = Some(timeout_handle);
        }

        log::info!("ReminderEngine: Session {} active again (post-snooze). Timeout timer started.", session_id);

        Ok(())
    }

    /// Creates a pending session, marks it triggered, and starts the timeout timer.
    async fn create_and_trigger_session(
        &self,
        pool: &Pool<Sqlite>,
        scheduler: SchedulerService,
    ) -> AppResult<()> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let settings = crate::services::SettingsService::load_all(pool).await.unwrap_or_default();

        let mut conn = pool.acquire().await?;

        // Create pending session.
        let session = SessionService::create_pending(
            pool,
            now_ms,
            settings.reminder_interval_minutes,
            settings.character_id.clone(),
            None,
        )
        .await?;

        log::info!("ReminderEngine: Created pending session ID {}.", session.id);

        // Transition pending -> triggered.
        SessionRepository::update_triggered(&mut conn, session.id, SessionStatus::Triggered, now_ms, now_ms).await?;
        log::info!("ReminderEngine: Session {} triggered.", session.id);

        let session_id = session.id;

        // Store active session and transition engine state.
        {
            let mut guard = match self.inner.lock() {
                Ok(g)  => g,
                Err(e) => return Err(crate::errors::AppError::Internal(e.to_string())),
            };
            guard.cancel_timeout();
            guard.active_session_id = Some(session_id);
            guard.engine_state      = EngineState::SessionActive;
        }

        // Emit events
        self.emit_trigger_events(session_id);

        // Start the session timeout timer.
        let timeout_handle = self.spawn_timeout_timer(
            pool.clone(),
            session_id,
            DEFAULT_SESSION_TIMEOUT_MINUTES,
            scheduler.clone(),
        );

        if let Ok(mut guard) = self.inner.lock() {
            guard.cancel_timeout();
            guard.timeout_handle = Some(timeout_handle);
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

            // Since it's a timeout timer, we don't have active_ms_today, so we pass -1
            if let Err(e) = engine.timeout_session(&pool, session_id, -1, &scheduler).await {
                log::error!("ReminderEngine: timeout_session failed for {}: {}", session_id, e);
            }
        });

        task.abort_handle()
    }

    /// Emits IPC trigger/presented events to the frontend.
    fn emit_trigger_events(&self, session_id: i64) {
        use tauri::Emitter;
        if let Ok(guard) = self.inner.lock() {
            if let Some(app) = &guard.app_handle {
                #[derive(serde::Serialize, Clone)]
                struct PresentedPayload {
                    #[serde(rename = "sessionId")]
                    session_id: String,
                }
                let payload = PresentedPayload {
                    session_id: session_id.to_string(),
                };
                app.emit("reminder:presented", payload).ok();

                #[derive(serde::Serialize, Clone)]
                struct TriggeredPayload {
                    #[serde(rename = "sessionId")]
                    session_id: String,
                    #[serde(rename = "dueAt")]
                    due_at: String,
                }
                let triggered_payload = TriggeredPayload {
                    session_id: session_id.to_string(),
                    due_at: chrono::Utc::now().to_rfc3339(),
                };
                app.emit("session:triggered", triggered_payload).ok();
            }
        }
    }

    /// Emits outcome events and presentation dismiss notifications.
    fn emit_outcome_events(
        &self,
        session_id: i64,
        status: &SessionStatus,
        snooze_delay_minutes: Option<i64>,
    ) {
        use tauri::Emitter;
        if let Ok(guard) = self.inner.lock() {
            if let Some(app) = &guard.app_handle {
                // 1. Emit the specific business event
                match status {
                    SessionStatus::Completed => {
                        #[derive(serde::Serialize, Clone)]
                        struct CompletedPayload {
                            #[serde(rename = "sessionId")]
                            session_id: String,
                        }
                        app.emit("session:completed", CompletedPayload {
                            session_id: session_id.to_string(),
                        }).ok();
                    }
                    SessionStatus::Snoozed => {
                        #[derive(serde::Serialize, Clone)]
                        struct SnoozedPayload {
                            #[serde(rename = "sessionId")]
                            session_id: String,
                            #[serde(rename = "durationMin")]
                            duration_min: i64,
                        }
                        let delay = snooze_delay_minutes.unwrap_or(crate::constants::DEFAULT_SNOOZE_DURATION_MINUTES);
                        app.emit("session:snoozed", SnoozedPayload {
                            session_id: session_id.to_string(),
                            duration_min: delay,
                        }).ok();
                    }
                    SessionStatus::TimedOut => {
                        #[derive(serde::Serialize, Clone)]
                        struct TimedOutPayload {
                            #[serde(rename = "sessionId")]
                            session_id: String,
                        }
                        app.emit("session:timedOut", TimedOutPayload {
                            session_id: session_id.to_string(),
                        }).ok();
                    }
                    SessionStatus::Ignored => {
                        #[derive(serde::Serialize, Clone)]
                        struct IgnoredPayload {
                            #[serde(rename = "sessionId")]
                            session_id: String,
                        }
                        app.emit("session:ignored", IgnoredPayload {
                            session_id: session_id.to_string(),
                        }).ok();
                    }
                    _ => {}
                }

                // 2. Emit the visual presentation dismiss event
                app.emit("reminder:dismissed", ()).ok();
            }
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
}

/// Reject invalid snooze durations before any timer or session state is changed.
fn validate_snooze_delay(delay_minutes: i64) -> AppResult<()> {
    // Keep backend input aligned with the approved SnoozeSelector options.
    if !matches!(delay_minutes, 5 | 10 | 15 | 30) {
        return Err(crate::errors::AppError::Validation(
            "Snooze duration must be one of: 5, 10, 15, or 30 minutes".into()
        ));
    }
    Ok(())
}

impl Default for ReminderEngineService {
    fn default() -> Self { Self::new() }
}

// -- Engine callback builder ---------------------------------------------------

/// Builds the `OnReminderDue` callback that wires the scheduler to the engine.
pub fn build_reminder_due_callback(
    engine:    ReminderEngineService,
    pool:      Pool<Sqlite>,
    scheduler: SchedulerService,
) -> OnReminderDue {
    Arc::new(move || {
        engine.handle_reminder_due(pool.clone(), scheduler.clone());
    })
}

#[cfg(test)]
mod phase_9a_tests {
    use super::validate_snooze_delay;

    #[test]
    fn snooze_duration_must_match_supported_options() {
        for duration in [5, 10, 15, 30] {
            assert!(validate_snooze_delay(duration).is_ok());
        }

        for duration in [-1, 0, 1, 4, 6, 60, i64::MAX] {
            assert!(validate_snooze_delay(duration).is_err());
        }
    }
}

