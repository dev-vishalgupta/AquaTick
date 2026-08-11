export type CharacterScale = "small" | "medium" | "large";
export type TargetMode = "automatic" | "manual";

export interface SettingsData {
  reminderIntervalMinutes: number;
  soundVolume: number;
  soundMuted: boolean;
  selectedCharacterId: string;
  characterScale: CharacterScale;
  startOnBoot: boolean;
  targetMode: TargetMode;
  manualTargetReminders: number;
}
