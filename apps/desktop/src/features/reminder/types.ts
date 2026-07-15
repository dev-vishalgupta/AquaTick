/**
 * @file types.ts
 *
 * Types for the Reminder Window subsystem.
 */

export interface ReminderWindowProps {
  /** Determines if the reminder window overlay is visible */
  isOpen: boolean;

  /** Indicates if the system is currently processing a click action */
  isProcessing: boolean;

  /** Callback triggered when the user drinks water */
  onDrink: () => void;

  /** Callback triggered when the user delays the reminder */
  onSnooze: (durationMinutes: number) => void;

  /** Callback triggered when the user skips/ignores the reminder */
  onIgnore: () => void;
}
