/**
 * @file PresentationTimelineCoordinator.types.ts
 *
 * Type definitions and contract declarations for Phase 8D/8E
 * Presentation Timeline Coordinator.
 */

import type { CharacterSystemAPI } from "../character/types";
import type { SoundSystemAPI } from "../sound/types";
import type { EventCoordinator } from "../coordinator";

/**
 * Character animation timeline states within the Presentation Layer.
 */
export type AnimationStage =
  | "Idle"
  | "Entering"
  | "Approaching"
  | "WaitingLoop"
  | "ResolvingDrink"
  | "ResolvingSnooze"
  | "ResolvingIgnore"
  | "Exiting"
  | "Complete";

/**
 * Audio playback timeline states within the Presentation Layer.
 */
export type AudioStage =
  | "Silent"
  | "PlayingFootstepsIn"
  | "PlayingPickup"
  | "PlayingNotification"
  | "AwaitingInput"
  | "PlayingDrink"
  | "PlayingFootstepsOut"
  | "Complete";

/**
 * Business outcome types that trigger presentation interruptions.
 */
export type ResolutionOutcome = "Drink" | "Snooze" | "Ignore" | "Timeout";

/**
 * Diagnostic log severity levels for Phase 8E diagnostic logging strategy.
 */
export type LogSeverity = "error" | "warn" | "info" | "debug";

/**
 * Configuration contract required by the PresentationTimelineCoordinator.
 */
export interface PresentationTimelineCoordinatorConfig {
  /**
   * Getter function returning the active CharacterSystemAPI instance or null if unavailable.
   */
  getCharacterController: () => CharacterSystemAPI | null;

  /**
   * SoundSystem instance. Defaults to singleton SoundSystem if omitted.
   */
  soundSystem?: SoundSystemAPI;

  /**
   * EventCoordinator singleton or instance. Defaults to global EventCoordinator if omitted.
   */
  eventCoordinator?: typeof EventCoordinator;

  /**
   * Maximum duration (in milliseconds) for the Presentation Recovery Watchdog in WaitingLoop.
   * Default: 600,000 ms (10 minutes).
   */
  watchdogDurationMs?: number;

  /**
   * Maximum duration (in milliseconds) to wait for Character System availability recovery.
   * Default: 3,000 ms.
   */
  characterMaxWaitMs?: number;

  /**
   * Interval (in milliseconds) between polling ticks during character availability recovery.
   * Default: 100 ms.
   */
  characterPollIntervalMs?: number;
}
