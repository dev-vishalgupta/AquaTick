/**
 * @file EventCoordinator.ts
 *
 * Singleton managing registration, routing, and lifecycle of Tauri backend events.
 * Coordinates validation and delivery of strongly typed payloads to frontend features.
 */

import { listen } from "@tauri-apps/api/event";
import type { EventType, EventPayload, EventHandler, UnsubscribeFn } from "./types";
import {
  validateSessionTriggered,
  validateSessionCompleted,
  validateSessionSnoozed,
  validateSessionTimedOut,
  validateSettingsChanged,
  validateCharacterChanged,
} from "./validators";

class EventCoordinatorClass {
  private handlers: { [K in EventType]?: Set<EventHandler<K>> } = {};
  private tauriUnlisteners: (() => void)[] = [];
  private isInitialized = false;

  /**
   * Subscribes a callback handler to a specific EventType.
   *
   * @param event    The event to listen for.
   * @param handler  The callback to execute when the event fires.
   * @returns        An UnsubscribeFn that detaches the listener.
   */
  on<T extends EventType>(event: T, handler: EventHandler<T>): UnsubscribeFn {
    if (!this.handlers[event]) {
      this.handlers[event] = new Set() as any;
    }
    const set = this.handlers[event] as Set<EventHandler<T>>;
    set.add(handler);

    return () => {
      this.off(event, handler);
    };
  }

  /**
   * Unsubscribes a callback handler from a specific EventType.
   */
  off<T extends EventType>(event: T, handler: EventHandler<T>): void {
    const set = this.handlers[event] as Set<EventHandler<T>> | undefined;
    if (set) {
      set.delete(handler);
    }
  }

  /**
   * Initializes connections to Tauri backend IPC events.
   * Ensures exactly one listener exists per event type.
   *
   * @throws If Tauri events fail to initialize.
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) return;

    try {
      // Set up Tauri listeners
      const unlistenTriggered = await listen<any>("session:triggered", (event) => {
        this.dispatch("session:triggered", event.payload, validateSessionTriggered);
      });
      this.tauriUnlisteners.push(unlistenTriggered);

      const unlistenCompleted = await listen<any>("session:completed", (event) => {
        this.dispatch("session:completed", event.payload, validateSessionCompleted);
      });
      this.tauriUnlisteners.push(unlistenCompleted);

      const unlistenSnoozed = await listen<any>("session:snoozed", (event) => {
        this.dispatch("session:snoozed", event.payload, validateSessionSnoozed);
      });
      this.tauriUnlisteners.push(unlistenSnoozed);

      const unlistenTimedOut = await listen<any>("session:timedOut", (event) => {
        this.dispatch("session:timedOut", event.payload, validateSessionTimedOut);
      });
      this.tauriUnlisteners.push(unlistenTimedOut);

      const unlistenSettings = await listen<any>("settings:changed", (event) => {
        this.dispatch("settings:changed", event.payload, validateSettingsChanged);
      });
      this.tauriUnlisteners.push(unlistenSettings);

      const unlistenCharacter = await listen<any>("character:changed", (event) => {
        this.dispatch("character:changed", event.payload, validateCharacterChanged);
      });
      this.tauriUnlisteners.push(unlistenCharacter);

      this.isInitialized = true;
    } catch (err) {
      // Clear any listeners that did get registered before failure
      this.destroy();
      throw new Error(`EventCoordinator: Failed to initialize Tauri event listeners. Raw error: ${err}`);
    }
  }

  /**
   * Destroys all active listeners and resets state.
   */
  destroy(): void {
    for (const unlistener of this.tauriUnlisteners) {
      unlistener();
    }
    this.tauriUnlisteners = [];
    this.isInitialized = false;
    this.handlers = {};
  }

  /**
   * Internal dispatcher. Validates raw payload data and executes active handlers.
   */
  private dispatch<T extends EventType>(
    event: T,
    rawPayload: any,
    validator: (data: any) => EventPayload<T>
  ): void {
    let validatedPayload: EventPayload<T>;
    try {
      validatedPayload = validator(rawPayload);
    } catch (valErr) {
      console.error(`EventCoordinator: Schema validation failed for event "${event}".`, valErr, rawPayload);
      return;
    }

    const set = this.handlers[event];
    if (set) {
      for (const handler of set) {
        try {
          handler(validatedPayload);
        } catch (handlerErr) {
          console.error(`EventCoordinator: Exception thrown in handler for event "${event}".`, handlerErr);
        }
      }
    }
  }

  /**
   * Package-private method to allow MockCoordinator to bypass Tauri IPC and
   * dispatch simulated events directly to listeners.
   */
  dispatchMock<T extends EventType>(event: T, payload: EventPayload<T>): void {
    const set = this.handlers[event] as Set<EventHandler<T>> | undefined;
    if (set) {
      for (const handler of set) {
        try {
          handler(payload);
        } catch (handlerErr) {
          console.error(`EventCoordinator: Exception thrown in mock handler for event "${event}".`, handlerErr);
        }
      }
    }
  }
}

export const EventCoordinator = new EventCoordinatorClass();
