import { useEffect, useState } from "react";
import { EventCoordinator } from "../coordinator";
import { getSettings } from "../settings/api/settings_api";
import { SoundSystem } from "../sound";
import OverlayCoordinator from "./OverlayCoordinator";
import type {
  ReminderPresentedPayload,
  UnsubscribeFn,
} from "../coordinator";
import type { SettingsData } from "../settings/types";

const EVENT_COORDINATOR_RETRY_DELAY_MS = 2_000;

export default function OverlayOrchestrator() {
  const [presentedPayload, setPresentedPayload] = useState<ReminderPresentedPayload | null>(null);
  const [isDismissed, setIsDismissed] = useState(false);
  const [settings, setSettings] = useState<SettingsData | null>(null);

  useEffect(() => {
    let active = true;
    let retryTimerId: ReturnType<typeof setTimeout> | null = null;
    let unsubs: UnsubscribeFn[] = [];

    const bindListeners = () => {
      if (!active) return;
      
      const unsubPresented = EventCoordinator.on("reminder:presented", (payload: ReminderPresentedPayload) => {
        console.info("[OverlayOrchestrator] reminder:presented event received", payload);
        if (active) {
          setPresentedPayload(payload);
          setIsDismissed(false);
        }
      });
      unsubs.push(unsubPresented);

      const unsubDismissed = EventCoordinator.on("reminder:dismissed", () => {
        console.info("[OverlayOrchestrator] reminder:dismissed event received");
        if (active) {
          setIsDismissed(true);
          setPresentedPayload(null);
        }
      });
      unsubs.push(unsubDismissed);

      const unsubSettings = EventCoordinator.on("settings:changed", (payload) => {
        console.info("[OverlayOrchestrator] settings:changed event received", payload);
        if (active && payload.settings) {
          setSettings(payload.settings);
        }
      });
      unsubs.push(unsubSettings);

      const unsubCharacter = EventCoordinator.on("character:changed", (payload) => {
        console.info("[OverlayOrchestrator] character:changed event received", payload);
        if (active && payload.characterId) {
          setSettings((prev) => {
            if (!prev) return null;
            return {
              ...prev,
              selectedCharacterId: payload.characterId,
            };
          });
        }
      });
      unsubs.push(unsubCharacter);
    };

    const init = async () => {
      let initialized = false;
      try {
        await EventCoordinator.initialize();
        initialized = true;
      } catch (err) {
        console.error(
          `[OverlayOrchestrator] EventCoordinator.initialize() failed. Scheduling retry in ${EVENT_COORDINATOR_RETRY_DELAY_MS}ms.`,
          err
        );

        // Schedule single automatic retry after EVENT_COORDINATOR_RETRY_DELAY_MS
        retryTimerId = setTimeout(async () => {
          if (!active) return;
          try {
            await EventCoordinator.initialize();
            console.info("[OverlayOrchestrator] EventCoordinator.initialize() retry succeeded.");
            bindListeners();
          } catch (retryErr) {
            console.error(
              "[OverlayOrchestrator] EventCoordinator.initialize() retry failed. Halting event listener setup.",
              retryErr
            );
          }
        }, EVENT_COORDINATOR_RETRY_DELAY_MS);
      }

      if (initialized) {
        bindListeners();
      }

      try {
        const s = await getSettings();
        if (active) setSettings(s);
      } catch (err) {
        console.error("[OverlayOrchestrator] failed to load settings.", err);
      }

      try {
        await SoundSystem.preloadAll();
      } catch (err) {
        console.warn("[OverlayOrchestrator] failed to preload sounds gracefully.", err);
      }
    };

    init();

    return () => {
      active = false;
      if (retryTimerId !== null) {
        clearTimeout(retryTimerId);
        retryTimerId = null;
      }
      for (const unsub of unsubs) {
        try {
          unsub();
        } catch {
          // Ignore individual unsubscription errors on teardown
        }
      }
      unsubs = [];
      EventCoordinator.destroy();
    };
  }, []);

  return (
    <OverlayCoordinator
      presentedPayload={presentedPayload}
      onPresentedConsumed={() => setPresentedPayload(null)}
      isDismissed={isDismissed}
      onDismissedConsumed={() => setIsDismissed(false)}
      settings={settings}
    />
  );
}
