import { invoke } from "@tauri-apps/api/core";
import type { SettingsData } from "../types";
import { mapBackendSettingsToFrontendSettings } from "./mappers";
import type { AppSettingsBackend } from "./mappers";

// Unified mapping from frontend SettingsData keys to backend settings keys
const SETTING_KEY_MAP: Record<keyof SettingsData, string> = {
  reminderIntervalMinutes: "reminder.interval_minutes",
  soundVolume: "reminder.sound_volume",
  soundMuted: "reminder.sound_enabled",
  selectedCharacterId: "character.id",
  characterScale: "character.scale",
  startOnBoot: "app.start_on_boot",
  targetMode: "target_mode",
  manualTargetReminders: "manual_target_reminders",
};

/**
 * Fetches settings from the SQLite database via Tauri IPC,
 * converting them into the frontend SettingsData format.
 */
export async function getSettings(): Promise<SettingsData> {
  const backendData = await invoke<AppSettingsBackend>("get_settings");
  return mapBackendSettingsToFrontendSettings(backendData);
}

/**
 * Updates a setting in the SQLite database via Tauri IPC.
 * Performs serialization and key/value mapping between React and Rust formats.
 */
export async function updateSetting<K extends keyof SettingsData>(
  key: K,
  value: SettingsData[K]
): Promise<void> {
  const backendKey = SETTING_KEY_MAP[key];
  if (!backendKey) {
    throw new Error(`[Settings API] Unknown setting key: ${key}`);
  }

  let backendValue: string;

  // Key-specific normalization and conversion
  if (key === "soundVolume") {
    // Map volume 0-100 to 0.0-1.0 real number
    const normalizedVolume = (value as number) / 100;
    backendValue = normalizedVolume.toString();
  } else if (key === "soundMuted") {
    // Map muted to enabled (inverted boolean)
    const soundEnabled = !(value as boolean);
    backendValue = soundEnabled.toString();
  } else if (typeof value === "boolean") {
    backendValue = value.toString();
  } else if (typeof value === "number") {
    backendValue = value.toString();
  } else {
    backendValue = String(value);
  }

  await invoke("update_setting", {
    key: backendKey,
    value: backendValue,
  });
}
