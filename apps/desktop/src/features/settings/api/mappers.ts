import type { SettingsData, CharacterScale, TargetMode } from "../types";

// Raw payload structure returned by the backend get_settings IPC command and settings:changed event
export interface AppSettingsBackend {
  schema_version: number;
  app_version: string;
  reminder_interval_minutes: number;
  reminder_enabled: boolean;
  sound_enabled: boolean;
  sound_volume: number;
  character_id: string;
  character_scale: string;
  start_on_boot: boolean;
  target_mode: string;
  manual_target_reminders: number;
}

/**
 * Maps backend AppSettingsBackend (snake_case) to frontend SettingsData (camelCase).
 */
export function mapBackendSettingsToFrontendSettings(backendData: AppSettingsBackend): SettingsData {
  return {
    reminderIntervalMinutes: backendData.reminder_interval_minutes,
    soundVolume: Math.round(backendData.sound_volume * 100),
    soundMuted: !backendData.sound_enabled,
    selectedCharacterId: backendData.character_id,
    characterScale: backendData.character_scale as CharacterScale,
    startOnBoot: backendData.start_on_boot,
    targetMode: backendData.target_mode as TargetMode,
    manualTargetReminders: backendData.manual_target_reminders,
  };
}
