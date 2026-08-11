/**
 * @file PresentationTimelineCoordinator.ts
 *
 * Framework-agnostic coordinator for Phase 8D & 8E — Audio & Animation Timeline.
 *
 * Coordinates the Character System and Sound System across the four-stage
 * presentation lifecycle (Entry -> Approach -> Waiting Loop -> Exit).
 * Driven exclusively by Event Bridge events without holding business logic.
 * Implements Phase 8E reliability, edge case recovery, and watchdog mechanisms.
 */

import type { CharacterSystemAPI } from "../character/types";
import type { SoundSystemAPI } from "../sound/types";
import { SoundSystem } from "../sound/SoundSystem";
import { EventCoordinator } from "../coordinator/EventCoordinator";
import type { UnsubscribeFn } from "../coordinator/types";
import type {
  AnimationStage,
  AudioStage,
  LogSeverity,
  PresentationTimelineCoordinatorConfig,
  ResolutionOutcome,
} from "./PresentationTimelineCoordinator.types";

const DEFAULT_WATCHDOG_DURATION_MS = 600_000; // 10 minutes
const DEFAULT_CHARACTER_MAX_WAIT_MS = 3_000; // 3 seconds
const DEFAULT_CHARACTER_POLL_INTERVAL_MS = 100; // 100 ms

export class PresentationTimelineCoordinator {
  private readonly getCharacterController: () => CharacterSystemAPI | null;
  private readonly soundSystem: SoundSystemAPI;
  private readonly eventCoordinator: typeof EventCoordinator;
  private readonly watchdogDurationMs: number;
  private readonly characterMaxWaitMs: number;
  private readonly characterPollIntervalMs: number;

  private animationStage: AnimationStage = "Idle";
  private audioStage: AudioStage = "Silent";
  private queuedOutcome: ResolutionOutcome | null = null;
  private isClosing = false;
  private hasEmittedClosed = false;
  private isDestroyed = false;
  private activeSessionId: string | null = null;
  private unsubscribers: UnsubscribeFn[] = [];

  // Recovery & Watchdog Timers (Phase 8E)
  private watchdogTimerId: ReturnType<typeof setTimeout> | null = null;
  private characterPollTimerId: ReturnType<typeof setInterval> | null = null;
  private pendingSessionId: string | null = null;
  private pendingSessionStartTime = 0;

  constructor(config: PresentationTimelineCoordinatorConfig) {
    this.getCharacterController = config.getCharacterController;
    this.soundSystem = config.soundSystem ?? SoundSystem;
    this.eventCoordinator = config.eventCoordinator ?? EventCoordinator;
    this.watchdogDurationMs = config.watchdogDurationMs ?? DEFAULT_WATCHDOG_DURATION_MS;
    this.characterMaxWaitMs = config.characterMaxWaitMs ?? DEFAULT_CHARACTER_MAX_WAIT_MS;
    this.characterPollIntervalMs =
      config.characterPollIntervalMs ?? DEFAULT_CHARACTER_POLL_INTERVAL_MS;
  }

  /**
   * Initializes event listeners and binds to Event Bridge channels.
   */
  initialize(): void {
    this.destroy(); // Ensure clean slate before subscribing
    this.isDestroyed = false;

    const unsubPresented = this.eventCoordinator.on("reminder:presented", (payload) => {
      this.handleReminderPresented(payload.sessionId);
    });
    this.unsubscribers.push(unsubPresented);

    const unsubDismissed = this.eventCoordinator.on("reminder:dismissed", () => {
      this.handleReminderDismissed();
    });
    this.unsubscribers.push(unsubDismissed);

    const unsubCompleted = this.eventCoordinator.on("session:completed", () => {
      this.handleResolution("Drink");
    });
    this.unsubscribers.push(unsubCompleted);

    const unsubSnoozed = this.eventCoordinator.on("session:snoozed", () => {
      this.handleResolution("Snooze");
    });
    this.unsubscribers.push(unsubSnoozed);

    const unsubTimedOut = this.eventCoordinator.on("session:timedOut", () => {
      this.handleResolution("Timeout");
    });
    this.unsubscribers.push(unsubTimedOut);

    // Support session:ignored if dispatched by EventCoordinator
    if (typeof (this.eventCoordinator as any).on === "function") {
      try {
        const unsubIgnored = (this.eventCoordinator as any).on("session:ignored", () => {
          this.handleResolution("Ignore");
        });
        if (typeof unsubIgnored === "function") {
          this.unsubscribers.push(unsubIgnored);
        }
      } catch {
        // Safe check if event type is not registered
      }
    }
  }

  /**
   * Host Notification method (Preferred Phase 8E Character Availability Recovery).
   * Host calls this when the CharacterCanvas mounts and its controller becomes ready.
   */
  notifyCharacterReady(): void {
    if (this.isDestroyed || !this.pendingSessionId) {
      return;
    }

    const character = this.getCharacterController();
    if (character) {
      this.clearCharacterPollTimer();
      const sessionId = this.pendingSessionId;
      const elapsedMs = Date.now() - this.pendingSessionStartTime;
      this.pendingSessionId = null;

      this.log(
        "info",
        `Character System availability recovery succeeded via host notification (session ${sessionId}, elapsed ${elapsedMs}ms).`
      );

      this.executeStage1(sessionId);
    }
  }

  /**
   * Returns active session ID.
   */
  getActiveSessionId(): string | null {
    return this.activeSessionId;
  }

  /**
   * Returns current animation stage for diagnostics/testing.
   */
  getAnimationStage(): AnimationStage {
    return this.animationStage;
  }

  /**
   * Returns current audio stage for diagnostics/testing.
   */
  getAudioStage(): AudioStage {
    return this.audioStage;
  }

  private isTerminated(): boolean {
    if (this.isDestroyed || this.isClosing) return true;
    const stage = this.animationStage as AnimationStage;
    return stage === "Complete" || stage === "Idle";
  }

  /**
   * Diagnostic logging helper following the Phase 8E severity model.
   */
  private log(severity: LogSeverity, message: string, ...extra: any[]): void {
    const formatted = `[PresentationTimelineCoordinator] ${message}`;
    if (severity === "error") {
      console.error(formatted, ...extra);
    } else if (severity === "warn") {
      console.warn(formatted, ...extra);
    } else if (severity === "info") {
      console.info(formatted, ...extra);
    } else {
      console.debug(formatted, ...extra);
    }
  }

  // -------------------------------------------------------------------------
  // Timeline Stages
  // -------------------------------------------------------------------------

  /**
   * Triggered by reminder:presented event.
   * Handles duplicate presentation protection & aborts active lifecycles if needed.
   */
  private handleReminderPresented(sessionId: string): void {
    if (this.isDestroyed) return;

    // Rule 2 & O-1: Single Active Presentation Protection.
    // If a presentation is already active, abort it cleanly before starting new session.
    if (this.animationStage !== "Idle" && this.animationStage !== "Complete") {
      this.log(
        "warn",
        `reminder:presented received while active — lifecycle aborted. (incoming session: ${sessionId}, aborted session: ${this.activeSessionId}, stage: ${this.animationStage})`
      );
      this.abortCurrentPresentation("replaced");
    }

    this.activeSessionId = sessionId;
    this.isClosing = false;
    this.hasEmittedClosed = false;
    this.queuedOutcome = null;

    const character = this.getCharacterController();

    // Section 9: Character Controller Availability Check & Recovery
    if (!character) {
      this.log(
        "info",
        `Character System unavailable at session start. Starting availability recovery for session ${sessionId}.`
      );

      this.pendingSessionId = sessionId;
      this.pendingSessionStartTime = Date.now();

      // Start Fallback Polling Timer in case host notification is not fired
      this.clearCharacterPollTimer();
      this.characterPollTimerId = setInterval(() => {
        this.checkCharacterAvailabilityPolling();
      }, this.characterPollIntervalMs);

      return;
    }

    this.executeStage1(sessionId);
  }

  /**
   * Polling tick fallback for character availability recovery.
   */
  private checkCharacterAvailabilityPolling(): void {
    if (this.isDestroyed || !this.pendingSessionId) {
      this.clearCharacterPollTimer();
      return;
    }

    const character = this.getCharacterController();
    const elapsedMs = Date.now() - this.pendingSessionStartTime;

    if (character) {
      this.clearCharacterPollTimer();
      const sessionId = this.pendingSessionId;
      this.pendingSessionId = null;

      this.log(
        "info",
        `Character System availability recovery succeeded via polling fallback (session ${sessionId}, elapsed ${elapsedMs}ms).`
      );

      this.executeStage1(sessionId);
      return;
    }

    if (elapsedMs >= this.characterMaxWaitMs) {
      this.clearCharacterPollTimer();
      const sessionId = this.pendingSessionId;
      this.pendingSessionId = null;

      this.log(
        "error",
        `Character System availability recovery timed out after ${elapsedMs}ms for session ${sessionId}.`
      );
      // Remain in Idle; do NOT emit presentation:closed, do NOT alter business state
    }
  }

  /**
   * Stage 1 — Entry Execution
   */
  private async executeStage1(sessionId: string): Promise<void> {
    if (this.isTerminated() || this.activeSessionId !== sessionId) {
      return;
    }

    const character = this.getCharacterController();
    if (!character) {
      this.log(
        "warn",
        `executeStage1: Character System unavailable for session ${sessionId}.`
      );
      return;
    }

    this.animationStage = "Entering";

    try {
      character.show();
    } catch (err) {
      this.log("error", "character.show() failed:", err);
    }

    // Audio: Entry footstep loop (Risk 2: audio failure does not block animation)
    this.safeAudioCall(() => {
      this.soundSystem.playLoop("footsteps");
      this.audioStage = "PlayingFootstepsIn";
    });

    // Walk to presentation position
    const targetX = typeof window !== "undefined" ? Math.round(window.innerWidth / 2) : 400;
    const defaultY = typeof window !== "undefined" ? Math.round(window.innerHeight - 120) : 300;

    try {
      await character.walkTo(targetX, defaultY);
    } catch (err) {
      this.log("warn", "walkTo interrupted or failed:", err);
    }

    if (this.isTerminated()) {
      return;
    }

    // Stop footsteps upon arriving at destination
    this.safeAudioCall(() => {
      this.soundSystem.stopLoop("footsteps");
    });

    await this.advanceToStage2();
  }

  /**
   * Stage 2 — Approach: Bottle pickup animation and audio
   */
  private async advanceToStage2(): Promise<void> {
    if (this.isTerminated()) {
      return;
    }

    this.animationStage = "Approaching";
    const character = this.getCharacterController();

    // Play pickup audio effect
    this.safeAudioCall(() => {
      this.soundSystem.play("pickup");
      this.audioStage = "PlayingPickup";
    });

    if (character) {
      try {
        await character.play("pickBottle");
      } catch (err) {
        this.log("warn", "play('pickBottle') failed:", err);
      }
    }

    if (this.isTerminated()) {
      return;
    }

    await this.advanceToStage3();
  }

  /**
   * Stage 3 — Waiting Loop: Looping drink animation & Presentation Recovery Watchdog
   */
  private async advanceToStage3(): Promise<void> {
    if (this.isTerminated()) {
      return;
    }

    this.animationStage = "WaitingLoop";
    const character = this.getCharacterController();

    // Play notification sound
    this.safeAudioCall(() => {
      this.soundSystem.play("notification");
      this.audioStage = "PlayingNotification";
    });

    if (character) {
      try {
        await character.play("drinkLoop", { loop: true });
      } catch (err) {
        this.log("warn", "play('drinkLoop') failed:", err);
      }
    }

    // Emit presentation:ready diagnostic acknowledgment
    try {
      await this.eventCoordinator.emit("presentation:ready", {
        componentId: "character-timeline",
      });
      this.log("debug", "presentation:ready emitted.", { sessionId: this.activeSessionId });
    } catch (err) {
      this.log("error", "failed to emit presentation:ready:", err);
    }

    // Start Presentation Recovery Watchdog (Phase 8E Section 10)
    this.clearWatchdogTimer();
    this.watchdogTimerId = setTimeout(() => {
      this.handleWatchdogTimeout();
    }, this.watchdogDurationMs);

    // Check if an outcome resolution was queued during Stage 1 or Stage 2
    if (this.queuedOutcome) {
      this.clearWatchdogTimer();
      const outcome = this.queuedOutcome;
      this.queuedOutcome = null;
      await this.advanceToStage4(outcome);
    } else {
      this.audioStage = "AwaitingInput";
    }
  }

  /**
   * Presentation Recovery Watchdog Handler (Section 10).
   * Protects Presentation Layer from hanging indefinitely on lost reminder:dismissed.
   */
  private handleWatchdogTimeout(): void {
    if (this.isTerminated() || this.animationStage !== "WaitingLoop") {
      return;
    }

    this.log(
      "warn",
      `Presentation Recovery Watchdog elapsed in WaitingLoop for session ${this.activeSessionId} after ${this.watchdogDurationMs}ms (reason: presentation-layer-recovery).`
    );

    // Initiate self-directed exit sequence (Stage 4) with Timeout outcome
    this.advanceToStage4("Timeout");
  }

  /**
   * Stage 4 — Resolution Interruption Handling
   */
  private handleResolution(outcome: ResolutionOutcome): void {
    if (this.animationStage === "Idle" || this.animationStage === "Complete" || this.isClosing) {
      return;
    }

    // Cancel Presentation Recovery Watchdog timer on valid resolution arrival
    this.clearWatchdogTimer();

    // Rule O-3: If outcome arrives during Entry or Approach, queue/overwrite it for Stage 3 entry
    if (this.animationStage === "Entering" || this.animationStage === "Approaching") {
      this.queuedOutcome = outcome;
      return;
    }

    if (this.animationStage === "WaitingLoop") {
      this.advanceToStage4(outcome);
    }
  }

  /**
   * Executes Stage 4 outcome animation sequence
   */
  private async advanceToStage4(outcome: ResolutionOutcome): Promise<void> {
    if (this.isTerminated()) {
      return;
    }

    this.clearWatchdogTimer();
    this.isClosing = true;
    const character = this.getCharacterController();

    if (outcome === "Drink") {
      this.animationStage = "ResolvingDrink";
      this.safeAudioCall(() => {
        this.soundSystem.play("drink");
        this.audioStage = "PlayingDrink";
      });
    } else if (outcome === "Snooze") {
      this.animationStage = "ResolvingSnooze";
    } else {
      this.animationStage = "ResolvingIgnore";
    }

    if (character) {
      try {
        await character.play("putBottleDown");
      } catch (err) {
        this.log("warn", "play('putBottleDown') failed:", err);
      }
    }

    await this.executeExitSequence();
  }

  /**
   * Stage 4 Exit Sequence: Footsteps audio loop, leave animation, and fallback handling
   */
  private async executeExitSequence(): Promise<void> {
    this.isClosing = true;
    this.animationStage = "Exiting";

    const character = this.getCharacterController();

    this.safeAudioCall(() => {
      this.soundSystem.playLoop("footsteps");
      this.audioStage = "PlayingFootstepsOut";
    });

    if (character) {
      try {
        await character.leave();
      } catch (err) {
        // Section 5 Rule 3: Safe recovery from failed character exit
        this.log("warn", `character.leave() failed for session ${this.activeSessionId}, calling fallback hide().`, err);
        try {
          character.hide();
        } catch (hideErr) {
          this.log("error", "character.hide() fallback failed:", hideErr);
        }
      }
    }

    this.safeAudioCall(() => {
      this.soundSystem.stopLoop("footsteps");
      this.audioStage = "Complete";
    });

    this.animationStage = "Complete";
    this.emitPresentationClosedOnce();
    this.resetState();
  }

  /**
   * Handles early dismissal via reminder:dismissed
   */
  private async handleReminderDismissed(): Promise<void> {
    if (this.isTerminated()) {
      return;
    }

    this.clearWatchdogTimer();

    // Early dismissal during Entry or Approach -> Instant cut to Hidden
    if (this.animationStage === "Entering" || this.animationStage === "Approaching") {
      this.isClosing = true;
      const character = this.getCharacterController();

      this.safeAudioCall(() => {
        this.soundSystem.stopLoop("footsteps");
        this.soundSystem.stop("pickup");
        this.audioStage = "Complete";
      });

      if (character) {
        try {
          character.hide();
        } catch (err) {
          this.log("warn", "hide() failed:", err);
        }
      }

      this.animationStage = "Complete";
      this.emitPresentationClosedOnce();
      this.resetState();
    } else if (this.animationStage === "WaitingLoop") {
      await this.advanceToStage4("Ignore");
    }
  }

  /**
   * Forcibly aborts an active presentation lifecycle due to replacement or teardown.
   */
  private abortCurrentPresentation(reason: string): void {
    this.isClosing = true;
    this.clearWatchdogTimer();
    this.clearCharacterPollTimer();

    this.safeAudioCall(() => {
      this.soundSystem.stopLoop("footsteps");
      this.soundSystem.stop("pickup");
      this.audioStage = "Complete";
    });

    const character = this.getCharacterController();
    if (character) {
      try {
        character.hide();
      } catch (err) {
        this.log("warn", "abortCurrentPresentation: character.hide() failed:", err);
      }
    }

    this.animationStage = "Complete";
    this.emitPresentationClosedOnce(reason);
    this.resetState();
  }

  // -------------------------------------------------------------------------
  // Helper Methods & Teardown
  // -------------------------------------------------------------------------

  /**
   * Emits presentation:closed exactly once per presentation lifecycle (Risk 4 mitigation).
   */
  private emitPresentationClosedOnce(reason?: string): void {
    if (this.hasEmittedClosed) return;
    this.hasEmittedClosed = true;

    try {
      this.eventCoordinator.emit("presentation:closed", {
        componentId: "character-timeline",
        ...(reason ? { reason } : {}),
      });
      this.log("debug", "presentation:closed emitted.", {
        componentId: "character-timeline",
        sessionId: this.activeSessionId,
        reason,
      });
    } catch (err) {
      this.log("error", "failed to emit presentation:closed:", err);
    }
  }

  /**
   * Safe wrapper for audio calls ensuring audio errors do not break animation (Risk 2 mitigation).
   */
  private safeAudioCall(action: () => void): void {
    try {
      action();
    } catch (err) {
      this.log("warn", "Audio call failed silently:", err);
    }
  }

  private clearWatchdogTimer(): void {
    if (this.watchdogTimerId !== null) {
      clearTimeout(this.watchdogTimerId);
      this.watchdogTimerId = null;
    }
  }

  private clearCharacterPollTimer(): void {
    if (this.characterPollTimerId !== null) {
      clearInterval(this.characterPollTimerId);
      this.characterPollTimerId = null;
    }
  }

  /**
   * Resets internal state to Idle/Silent.
   */
  private resetState(): void {
    this.clearWatchdogTimer();
    this.clearCharacterPollTimer();
    this.animationStage = "Idle";
    this.audioStage = "Silent";
    this.queuedOutcome = null;
    this.isClosing = false;
    this.pendingSessionId = null;
  }

  /**
   * Destroys all Event Bridge subscriptions and resets internal state (Section 11 Cleanup).
   */
  destroy(): void {
    this.isDestroyed = true;
    this.clearWatchdogTimer();
    this.clearCharacterPollTimer();

    if (this.activeSessionId && this.animationStage !== "Idle" && this.animationStage !== "Complete") {
      this.log(
        "info",
        `destroy() called during active presentation (session ${this.activeSessionId}, animationStage ${this.animationStage}).`
      );
    }

    for (const unsub of this.unsubscribers) {
      try {
        unsub();
      } catch (err) {
        // Individual catch ensures a failed unsubscription does not block remaining cleanups
        this.log("warn", "Error unsubscribing event listener:", err);
      }
    }
    this.unsubscribers = [];

    this.safeAudioCall(() => {
      this.soundSystem.stopLoop("footsteps");
    });

    const character = this.getCharacterController();
    if (character && this.animationStage !== "Idle" && this.animationStage !== "Complete") {
      try {
        character.hide();
      } catch {
        // Safe check on teardown
      }
    }

    this.resetState();
  }
}
