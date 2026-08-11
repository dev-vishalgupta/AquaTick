import { useState, useEffect } from "react";
import SettingsSection from "./SettingsSection";
import SectionCard from "./SectionCard";
import AboutSection from "./AboutSection";
import CharacterPreview from "./CharacterPreview";
import ToggleControl from "../../../components/ui/ToggleControl";
import SliderControl from "../../../components/ui/SliderControl";
import NumberInputControl from "../../../components/ui/NumberInputControl";
import SelectControl from "../../../components/ui/SelectControl";
import type { SettingsData } from "../types";
import { getSettings, updateSetting } from "../api/settings_api";

export default function SettingsPage() {
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getSettings()
      .then((data) => {
        if (active) {
          setSettings(data);
          setIsLoading(false);
        }
      })
      .catch((err) => {
        console.error("Failed to load settings:", err);
        if (active) {
          setError(String(err));
          setIsLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, []);

  const handleUpdate = async <K extends keyof SettingsData>(key: K, value: SettingsData[K]) => {
    try {
      await updateSetting(key, value);
      setSettings((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          [key]: value,
        };
      });
    } catch (err) {
      console.error(`Failed to update setting ${key}:`, err);
      // We could set an error banner or toast, but console.error is sufficient for core IPC flow.
    }
  };

  const targetModeOptions = [
    { value: "automatic", label: "Automatic (Based on active hours)" },
    { value: "manual", label: "Manual Target" },
  ];

  const characterOptions = [
    { value: "female", label: "Female (Default)" },
    { value: "male", label: "Male" },
  ];

  const scaleOptions = [
    { value: "small", label: "Small" },
    { value: "medium", label: "Medium" },
    { value: "large", label: "Large" },
  ];

  if (isLoading) {
    return (
      <div
        role="status"
        aria-label="Loading settings"
        className="w-full h-full flex items-center justify-center bg-slate-50 dark:bg-slate-950 text-slate-500 dark:text-slate-400"
      >
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 dark:border-primary-400" />
      </div>
    );
  }

  if (error || !settings) {
    return (
      <div
        role="alert"
        className="w-full h-full flex flex-col items-center justify-center bg-slate-50 dark:bg-slate-950 text-slate-500 dark:text-slate-400 space-y-4"
      >
        <p className="text-red-500 font-medium">Failed to load settings</p>
        <p className="text-sm">{error || "Settings not initialized"}</p>
      </div>
    );
  }

  return (
    <div className="w-full h-full max-h-screen overflow-y-auto custom-scrollbar px-4 py-6 md:px-8 space-y-8 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 transition-colors">
      <div className="max-w-2xl mx-auto space-y-6 pb-12">
        {/* Page Header */}
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-800 dark:text-slate-100">
            Settings
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400 mt-1">
            Customize your hydration companion and application preferences.
          </p>
        </div>

        {/* General Settings */}
        <SettingsSection
          title="General"
          description="Configure application boot behavior and daily hydration targets."
        >
          <SectionCard>
            <SelectControl
              id="target-mode"
              label="Daily Target Calculation Mode"
              options={targetModeOptions}
              value={settings.targetMode}
              onChange={(val) => handleUpdate("targetMode", val as any)}
            />
            {settings.targetMode === "manual" && (
              <>
                <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
                <NumberInputControl
                  id="manual-target-reminders"
                  label="Daily Reminder Target"
                  value={settings.manualTargetReminders}
                  min={1}
                  max={50}
                  onChange={(val) => handleUpdate("manualTargetReminders", val)}
                />
              </>
            )}
            <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
            <ToggleControl
              id="start-on-boot"
              label="Launch on Startup"
              description="Automatically start AquaTick when you log into your system."
              checked={settings.startOnBoot}
              onChange={(checked) => handleUpdate("startOnBoot", checked)}
            />
          </SectionCard>
        </SettingsSection>

        {/* Reminder Settings */}
        <SettingsSection
          title="Reminders & Notifications"
          description="Control timing and notification alerts."
        >
          <SectionCard>
            <NumberInputControl
              id="reminder-interval"
              label="Reminder Interval (minutes)"
              value={settings.reminderIntervalMinutes}
              min={5}
              max={360}
              onChange={(val) => handleUpdate("reminderIntervalMinutes", val)}
            />
            <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
            <SliderControl
              id="sound-volume"
              label="Notification Volume"
              min={0}
              max={100}
              value={settings.soundVolume}
              onChange={(val) => handleUpdate("soundVolume", val)}
              formatValue={(val) => `${val}%`}
            />
            <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
            <ToggleControl
              id="sound-muted"
              label="Mute Audio Alerts"
              description="Disable reminder sound effects completely."
              checked={settings.soundMuted}
              onChange={(checked) => handleUpdate("soundMuted", checked)}
            />
          </SectionCard>
        </SettingsSection>

        {/* Character Customization */}
        <SettingsSection
          title="Character Customization"
          description="Choose and resize your hydration desktop companion."
        >
          <SectionCard>
            <SelectControl
              id="character-select"
              label="Selected Character"
              options={characterOptions}
              value={settings.selectedCharacterId}
              onChange={(val) => handleUpdate("selectedCharacterId", val)}
            />
            <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
            <SelectControl
              id="character-scale"
              label="Companion Scale"
              options={scaleOptions}
              value={settings.characterScale}
              onChange={(val) => handleUpdate("characterScale", val as any)}
            />
            <div className="border-t border-slate-100 dark:border-slate-800/60 my-2" />
            {/* Visual character viewport preview */}
            <CharacterPreview
              characterId={settings.selectedCharacterId}
              scale={settings.characterScale}
            />
          </SectionCard>
        </SettingsSection>

        {/* About App */}
        <SettingsSection title="About">
          <SectionCard>
            <AboutSection />
          </SectionCard>
        </SettingsSection>
      </div>
    </div>
  );
}
