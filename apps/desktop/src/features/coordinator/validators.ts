/**
 * @file validators.ts
 *
 * Runtime schema validators for incoming Tauri JSON payloads.
 * Functions check data structure shape and types, throwing descriptive errors
 * if validation fails.
 */

import type {
  SessionTriggeredPayload,
  SessionCompletedPayload,
  SessionSnoozedPayload,
  SessionTimedOutPayload,
  SettingsChangedPayload,
  CharacterChangedPayload,
} from "./types";

export function validateSessionTriggered(data: any): SessionTriggeredPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (typeof data.sessionId !== "string") {
    throw new Error("sessionId must be a string");
  }
  if (typeof data.dueAt !== "string") {
    throw new Error("dueAt must be a string");
  }
  return {
    sessionId: data.sessionId,
    dueAt: data.dueAt,
  };
}

export function validateSessionCompleted(data: any): SessionCompletedPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (typeof data.sessionId !== "string") {
    throw new Error("sessionId must be a string");
  }
  return {
    sessionId: data.sessionId,
  };
}

export function validateSessionSnoozed(data: any): SessionSnoozedPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (typeof data.sessionId !== "string") {
    throw new Error("sessionId must be a string");
  }
  if (typeof data.durationMin !== "number") {
    throw new Error("durationMin must be a number");
  }
  return {
    sessionId: data.sessionId,
    durationMin: data.durationMin,
  };
}

export function validateSessionTimedOut(data: any): SessionTimedOutPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (typeof data.sessionId !== "string") {
    throw new Error("sessionId must be a string");
  }
  return {
    sessionId: data.sessionId,
  };
}

export function validateSettingsChanged(data: any): SettingsChangedPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (!data.settings || typeof data.settings !== "object") {
    throw new Error("settings field must be an object");
  }
  return {
    settings: data.settings,
  };
}

export function validateCharacterChanged(data: any): CharacterChangedPayload {
  if (!data || typeof data !== "object") {
    throw new Error("Payload must be an object");
  }
  if (typeof data.characterId !== "string") {
    throw new Error("characterId must be a string");
  }
  return {
    characterId: data.characterId,
  };
}
