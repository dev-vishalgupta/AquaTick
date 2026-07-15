import { useState } from "react";

/**
 * Hook managing local visual states for the Reminder Window UI.
 */
export function useReminderUI() {
  const [isSnoozeOpen, setIsSnoozeOpen] = useState(false);

  const toggleSnooze = () => setIsSnoozeOpen((prev) => !prev);
  const closeSnooze = () => setIsSnoozeOpen(false);

  return {
    isSnoozeOpen,
    toggleSnooze,
    closeSnooze,
  };
}
