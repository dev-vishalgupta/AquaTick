import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EventCoordinator } from "../features/coordinator";
import { CharacterCanvas } from "../features/character";
import type { CharacterCanvasHandle } from "../features/character";
import { ReminderWindow } from "../features/reminder";
import { SoundSystem } from "../features/sound";
import type {
  SessionTriggeredPayload,
  SessionCompletedPayload,
  SessionSnoozedPayload,
  SessionTimedOutPayload,
} from "../features/coordinator";

export default function SystemOrchestrator() {
  const canvasRef = useRef<CharacterCanvasHandle>(null);

  // Reminder Window state
  const [isOpen, setIsOpen] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    // Fetch and set character on startup
    const initCharacter = async () => {
      try {
        const settings = await invoke<any>("get_settings");
        if (settings && settings.character_id && canvasRef.current?.character && active) {
          await canvasRef.current.character.setCharacter(settings.character_id);
        }
      } catch (err) {
        console.error("SystemOrchestrator: failed to initialize character settings.", err);
      }
    };

    // Preload sound assets
    const initSound = async () => {
      try {
        await SoundSystem.preloadAll();
      } catch (err) {
        console.error("SystemOrchestrator: failed to preload sounds.", err);
      }
    };

    // Initialize EventCoordinator
    const initCoordinator = async () => {
      try {
        await EventCoordinator.initialize();
      } catch (err) {
        console.error("SystemOrchestrator: failed to initialize EventCoordinator.", err);
      }
    };

    initCoordinator().then(() => {
      initCharacter();
      initSound();
    });

    // Subscriptions
    const unsubscribeTriggered = EventCoordinator.on(
      "session:triggered",
      async (payload: SessionTriggeredPayload) => {
        if (!active) return;
        console.log("SystemOrchestrator: session:triggered event received", payload);

        setActiveSessionId(payload.sessionId);

        // Play notification sound when reminder appears
        SoundSystem.play("notification");

        const char = canvasRef.current?.character;
        if (char) {
          // Step 1: Make character visible at default start pos
          char.show();

          // Step 2: Calculate walk coordinates (walk to center of viewport)
          const walkTargetX = window.innerWidth / 2;
          const walkTargetY = window.innerHeight - 150; // offset slightly from bottom

          // Step 3: Walk to center (with footsteps sound loop)
          SoundSystem.playLoop("footsteps");
          await char.walkTo(walkTargetX, walkTargetY);
          SoundSystem.stopLoop("footsteps");

          // Step 4: Pick up bottle (with pickup sound triggered on frame 2)
          await char.play("pickBottle", {
            onFrame: (frameIndex) => {
              if (frameIndex === 2) {
                SoundSystem.play("pickup");
              }
            },
          });

          // Step 5: Loop drinking animation (with drink loop sound) and reveal Reminder Window
          SoundSystem.playLoop("drink");
          await char.play("drinkLoop");
          setIsOpen(true);
        } else {
          // Fallback: if character system fails, just show the window immediately
          setIsOpen(true);
        }
      }
    );

    const handleSessionEnded = async () => {
      if (!active) return;

      // Stop drink loop sound
      SoundSystem.stopLoop("drink");

      setIsOpen(false);
      setIsProcessing(false);

      const char = canvasRef.current?.character;
      if (char) {
        // Step 1: Put bottle down (with click sound triggered on frame 3)
        await char.play("putBottleDown", {
          onFrame: (frameIndex) => {
            if (frameIndex === 3) {
              SoundSystem.play("click");
            }
          },
        });
        // Step 2: Walk off-screen and hide (with footsteps sound loop)
        SoundSystem.playLoop("footsteps");
        await char.leave();
        SoundSystem.stopLoop("footsteps");
      }
      setActiveSessionId(null);
    };

    const unsubscribeCompleted = EventCoordinator.on(
      "session:completed",
      (payload: SessionCompletedPayload) => {
        console.log("SystemOrchestrator: session:completed event received", payload);
        handleSessionEnded();
      }
    );

    const unsubscribeSnoozed = EventCoordinator.on(
      "session:snoozed",
      (payload: SessionSnoozedPayload) => {
        console.log("SystemOrchestrator: session:snoozed event received", payload);
        handleSessionEnded();
      }
    );

    const unsubscribeTimedOut = EventCoordinator.on(
      "session:timedOut",
      (payload: SessionTimedOutPayload) => {
        console.log("SystemOrchestrator: session:timedOut event received", payload);
        handleSessionEnded();
      }
    );

    const unsubscribeCharacterChanged = EventCoordinator.on(
      "character:changed",
      async (payload) => {
        if (!active) return;
        console.log("SystemOrchestrator: character:changed event received", payload);
        const char = canvasRef.current?.character;
        if (char) {
          try {
            await char.setCharacter(payload.characterId);
          } catch (err) {
            console.error("SystemOrchestrator: failed to swap character", err);
          }
        }
      }
    );

    const unsubscribeSettingsChanged = EventCoordinator.on(
      "settings:changed",
      async (payload) => {
        if (!active) return;
        console.log("SystemOrchestrator: settings:changed event received", payload);
        const char = canvasRef.current?.character;
        if (char && payload.settings) {
          const newCharId =
            payload.settings.character_id || payload.settings.selectedCharacterId;
          if (newCharId) {
            try {
              await char.setCharacter(newCharId);
            } catch (err) {
              console.error(
                "SystemOrchestrator: failed to swap character from settings change",
                err
              );
            }
          }
        }
      }
    );

    return () => {
      active = false;
      unsubscribeTriggered();
      unsubscribeCompleted();
      unsubscribeSnoozed();
      unsubscribeTimedOut();
      unsubscribeCharacterChanged();
      unsubscribeSettingsChanged();
      EventCoordinator.destroy();

      // Clean up active sound loops on unmount
      SoundSystem.stopLoop("footsteps");
      SoundSystem.stopLoop("drink");
    };
  }, []);

  const handleDrink = async () => {
    if (!activeSessionId) return;
    SoundSystem.play("click");
    setIsProcessing(true);
    try {
      await invoke("complete_session", { id: parseInt(activeSessionId) });
    } catch (err) {
      console.error("SystemOrchestrator: complete_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  const handleSnooze = async (durationMinutes: number) => {
    if (!activeSessionId) return;
    SoundSystem.play("click");
    setIsProcessing(true);
    try {
      await invoke("snooze_session", {
        id: parseInt(activeSessionId),
        delayMinutes: durationMinutes,
      });
    } catch (err) {
      console.error("SystemOrchestrator: snooze_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  const handleIgnore = async () => {
    if (!activeSessionId) return;
    SoundSystem.play("click");
    setIsProcessing(true);
    try {
      await invoke("timeout_session", { id: parseInt(activeSessionId) });
    } catch (err) {
      console.error("SystemOrchestrator: timeout_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  return (
    <>
      <CharacterCanvas ref={canvasRef} />
      <ReminderWindow
        isOpen={isOpen}
        isProcessing={isProcessing}
        onDrink={handleDrink}
        onSnooze={handleSnooze}
        onIgnore={handleIgnore}
      />
    </>
  );
}
