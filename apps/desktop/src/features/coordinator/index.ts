/**
 * @file index.ts
 *
 * Public barrel export for the Event Coordinator module.
 */

export { EventCoordinator } from "./EventCoordinator";
export { MockCoordinator } from "./MockCoordinator";
export type {
  EventType,
  EventPayload,
  SessionTriggeredPayload,
  SessionCompletedPayload,
  SessionSnoozedPayload,
  SessionTimedOutPayload,
  SettingsChangedPayload,
  CharacterChangedPayload,
  AppSettings,
  UnsubscribeFn,
} from "./types";
export {
  validateSessionTriggered,
  validateSessionCompleted,
  validateSessionSnoozed,
  validateSessionTimedOut,
  validateSettingsChanged,
  validateCharacterChanged,
} from "./validators";
