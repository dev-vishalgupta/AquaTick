import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CharacterCanvas } from "../character";
import type { CharacterCanvasHandle } from "../character/components/CharacterCanvas";
import { ReminderWindow } from "../reminder";
import { EventCoordinator } from "../coordinator";
import type { ReminderPresentedPayload } from "../coordinator";
import type { SettingsData } from "../settings/types";
import { PresentationTimelineCoordinator } from "./PresentationTimelineCoordinator";

interface OverlayCoordinatorProps {
  presentedPayload: ReminderPresentedPayload | null;
  onPresentedConsumed: () => void;
  isDismissed: boolean;
  onDismissedConsumed: () => void;
  settings: SettingsData | null;
}

export default function OverlayCoordinator({
  presentedPayload,
  onPresentedConsumed,
  isDismissed,
  onDismissedConsumed,
  settings,
}: OverlayCoordinatorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  const canvasRef = useRef<CharacterCanvasHandle | null>(null);
  const timelineCoordinatorRef = useRef<PresentationTimelineCoordinator | null>(null);

  // Initialize PresentationTimelineCoordinator for Audio & Animation sequencing (Phase 8D & 8E)
  useEffect(() => {
    const coordinator = new PresentationTimelineCoordinator({
      getCharacterController: () => canvasRef.current?.character ?? null,
    });
    coordinator.initialize();
    timelineCoordinatorRef.current = coordinator;

    // Event-driven Host Notification: Notify coordinator if character controller is already ready
    if (canvasRef.current?.character) {
      coordinator.notifyCharacterReady();
    }

    return () => {
      coordinator.destroy();
      timelineCoordinatorRef.current = null;
    };
  }, []);

  // Event-driven Host Notification: Trigger notifyCharacterReady whenever settings change or canvas ref connects
  useEffect(() => {
    if (canvasRef.current?.character) {
      timelineCoordinatorRef.current?.notifyCharacterReady();
    }
  }, [settings]);

  // Handle session presented (starts showing window)
  useEffect(() => {
    if (!presentedPayload) return;
    const payload = presentedPayload;
    onPresentedConsumed();

    const showSequence = async () => {
      setActiveSessionId(payload.sessionId);
      setIsOpen(true);

      // Emit optional diagnostic PresentationReady event for overlay window
      try {
        await EventCoordinator.emit("presentation:ready", { componentId: "overlay" });
      } catch (err) {
        console.error("OverlayCoordinator: failed to emit presentation:ready", err);
      }
    };

    showSequence();
  }, [presentedPayload, onPresentedConsumed]);

  // Handle session dismissed (starts hiding window)
  useEffect(() => {
    if (!isDismissed) return;
    onDismissedConsumed();

    const hideSequence = async () => {
      setIsOpen(false);
      setIsProcessing(false);

      // Emit PresentationClosed to signal that overlay presentation has fully hidden
      try {
        await EventCoordinator.emit("presentation:closed", { componentId: "overlay" });
      } catch (err) {
        console.error("OverlayCoordinator: failed to emit presentation:closed", err);
      }

      setActiveSessionId(null);
    };

    hideSequence();
  }, [isDismissed, onDismissedConsumed]);

  const handleDrink = async () => {
    if (!activeSessionId) return;
    setIsProcessing(true);
    try {
      await invoke("complete_session", { id: parseInt(activeSessionId) });
    } catch (err) {
      console.error("OverlayCoordinator: complete_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  const handleSnooze = async (durationMinutes: number) => {
    if (!activeSessionId) return;
    setIsProcessing(true);
    try {
      await invoke("snooze_session", {
        id: parseInt(activeSessionId),
        delayMinutes: durationMinutes,
      });
    } catch (err) {
      console.error("OverlayCoordinator: snooze_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  const handleIgnore = async () => {
    if (!activeSessionId) return;
    setIsProcessing(true);
    try {
      await invoke("ignore_session", { id: parseInt(activeSessionId) });
    } catch (err) {
      console.error("OverlayCoordinator: ignore_session invoke failed", err);
      setIsProcessing(false);
    }
  };

  return (
    <>
      <CharacterCanvas ref={canvasRef} settings={settings} />
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
