/**
 * @file MockCoordinator.ts
 *
 * Developer tool for triggering simulated events directly in the EventCoordinator.
 * Allows frontend manual and visual testing without relying on the Tauri IPC layer.
 */

import { EventCoordinator } from "./EventCoordinator";
import type { EventType, EventPayload } from "./types";

export class MockCoordinator {
  /**
   * Dispatches a simulated event directly to all registered handlers for testing.
   *
   * @param event    The EventType to simulate.
   * @param payload  The payload matching the event requirements.
   */
  static trigger<T extends EventType>(event: T, payload: EventPayload<T>): void {
    EventCoordinator.dispatchMock(event, payload);
  }
}
