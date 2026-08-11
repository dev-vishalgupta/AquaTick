/**
 * @file types.ts
 *
 * Domain types and payload definitions for the Event Coordinator.
 */

export type EventType =
  | "session:triggered"
  | "session:completed"
  | "session:snoozed"
  | "session:timedOut"
  | "settings:changed"
  | "character:changed"
  | "reminder:presented"
  | "presentation:ready"
  | "reminder:dismissed"
  | "presentation:closed";

export interface SessionTriggeredPayload {
  sessionId: string;
  dueAt: string;
}

export interface SessionCompletedPayload {
  sessionId: string;
}

export interface SessionSnoozedPayload {
  sessionId: string;
  durationMin: number;
}

export interface SessionTimedOutPayload {
  sessionId: string;
}

export interface ReminderPresentedPayload {
  sessionId: string;
}

export interface PresentationReadyPayload {
  componentId: string;
}

export interface ReminderDismissedPayload {}

export interface PresentationClosedPayload {
  componentId: string;
  reason?: string;
}

import type { SettingsData } from "../settings/types";

export interface AppSettings extends SettingsData {
  [key: string]: any;
}

export interface SettingsChangedPayload {
  settings: AppSettings;
}

export interface CharacterChangedPayload {
  characterId: string;
}

/**
 * Maps EventType strings to their strictly typed payload models.
 */
export interface EventPayloadMap {
  "session:triggered": SessionTriggeredPayload;
  "session:completed": SessionCompletedPayload;
  "session:snoozed": SessionSnoozedPayload;
  "session:timedOut": SessionTimedOutPayload;
  "settings:changed": SettingsChangedPayload;
  "character:changed": CharacterChangedPayload;
  "reminder:presented": ReminderPresentedPayload;
  "presentation:ready": PresentationReadyPayload;
  "reminder:dismissed": ReminderDismissedPayload;
  "presentation:closed": PresentationClosedPayload;
}

export type EventPayload<T extends EventType> = EventPayloadMap[T];

export type EventHandler<T extends EventType> = (payload: EventPayload<T>) => void;

export type UnsubscribeFn = () => void;
