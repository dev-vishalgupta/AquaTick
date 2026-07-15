/**
 * @file MovementController.ts
 *
 * Deterministic spatial interpolation for character movement.
 *
 * # Responsibilities (this file)
 *   - Track the character's current logical position (CSS pixels).
 *   - Accept a movement target and speed, then advance position each tick
 *     toward that target.
 *   - Detect arrival (destination reached) and signal the caller.
 *   - Prevent overshooting by snapping to the exact target when the
 *     remaining distance is smaller than the tick's step size.
 *
 * # Non-responsibilities (enforced)
 *   - No rendering or drawing of any kind.
 *   - No animation frame tracking.
 *   - No requestAnimationFrame or timer ownership.
 *   - No state machine transitions.
 *   - No React or Tauri dependencies.
 *
 * # Movement model
 *   On each tick the controller computes the maximum travel distance for that
 *   tick:
 *
 *     maxStep = speed (px/s) × deltaMs / 1000
 *
 *   It then moves the position along the straight-line vector toward the
 *   target by exactly `maxStep` pixels, or snaps to the target if the
 *   remaining distance is ≤ maxStep.
 *
 *   The direction vector is normalised before scaling so diagonal movement
 *   always advances at the configured speed (no hypotenuse inflation).
 *
 *   All coordinates are in logical CSS pixels. The RenderEngine applies DPI
 *   correction when drawing — the MovementController never needs to know the
 *   device pixel ratio.
 */

import type { Position } from "../types";
import { DEFAULT_WALK_SPEED_PX_PER_SEC } from "../constants";

// ---------------------------------------------------------------------------
// MovementController
// ---------------------------------------------------------------------------

/**
 * MovementController
 *
 * Advances the character's position toward a target coordinate each tick.
 * One instance is owned by the CharacterController (Phase 5A.6).
 *
 * @example
 * ```ts
 * const movement = new MovementController();
 * movement.setPosition(startX, startY);
 * movement.moveTo(targetX, targetY, 120);   // 120 px/s
 *
 * // Inside the render loop:
 * const arrived = movement.tick(deltaMs);
 * const { x, y } = movement.getPosition();
 * engine.drawFrame(sheet, meta, animDef, frameIndex, x, y, scale);
 * ```
 */
export class MovementController {
  // -------------------------------------------------------------------------
  // Private state
  // -------------------------------------------------------------------------

  /** Current position in logical CSS pixels. */
  private position: Position = { x: 0, y: 0 };

  /** Movement target. Null when no movement is in progress. */
  private target: Position | null = null;

  /** Movement speed in logical pixels per second. */
  private speed: number = DEFAULT_WALK_SPEED_PX_PER_SEC;

  /**
   * True once the controller has arrived at the most recent `moveTo` target.
   *
   * Stays true until `moveTo()`, `setPosition()`, or `stop()` is called.
   */
  private arrived: boolean = false;

  // -------------------------------------------------------------------------
  // Public API — setup
  // -------------------------------------------------------------------------

  /**
   * Teleports the character to `(x, y)` instantly without movement.
   *
   * Clears any active target and resets the arrived flag.
   * Use this to place the character at the entry origin before calling
   * `moveTo()`.
   *
   * @param x  Logical X coordinate (CSS pixels).
   * @param y  Logical Y coordinate (CSS pixels).
   */
  setPosition(x: number, y: number): void {
    this.position = { x, y };
    this.target = null;
    this.arrived = false;
  }

  /**
   * Begins moving the character toward `(x, y)` at the given speed.
   *
   * Resets the arrived flag so `hasArrived()` returns `false` until the
   * new target is reached.
   *
   * If the target is the same as the current position, the controller will
   * arrive on the very next `tick()` call.
   *
   * @param x      Target X coordinate in logical CSS pixels.
   * @param y      Target Y coordinate in logical CSS pixels.
   * @param speed  Movement speed in logical pixels per second.
   *               Clamped to a minimum of 1 px/s. Defaults to
   *               `DEFAULT_WALK_SPEED_PX_PER_SEC`.
   */
  moveTo(x: number, y: number, speed?: number): void {
    this.target = { x, y };
    // Clamp to prevent a zero-speed infinite movement loop.
    this.speed = Math.max(speed ?? DEFAULT_WALK_SPEED_PX_PER_SEC, 1);
    this.arrived = false;
  }

  // -------------------------------------------------------------------------
  // Public API — tick
  // -------------------------------------------------------------------------

  /**
   * Advances the character's position toward the current target.
   *
   * Must be called on every iteration of the render loop while movement is
   * active.
   *
   * @param deltaMs  Milliseconds elapsed since the previous `tick()` call.
   *                 Must be ≥ 0. Negative values are treated as zero.
   * @returns        `true` exactly once — on the tick when the target is
   *                 reached or passed. Returns `false` for all other ticks,
   *                 including ticks with no active target.
   */
  tick(deltaMs: number): boolean {
    if (this.target === null || this.arrived) return false;

    const safeDelta = Math.max(0, deltaMs);
    const maxStep = this.speed * (safeDelta / 1000);

    const dx = this.target.x - this.position.x;
    const dy = this.target.y - this.position.y;
    const distance = Math.sqrt(dx * dx + dy * dy);

    // Snap to target when remaining distance fits within this tick's step.
    // This prevents overshooting and ensures exact arrival coordinates.
    if (distance <= maxStep) {
      this.position = { x: this.target.x, y: this.target.y };
      this.arrived = true;
      return true;
    }

    // Advance by maxStep along the normalised direction vector.
    // Dividing by distance produces a unit vector; scaling by maxStep gives
    // exactly maxStep pixels of travel in the correct direction.
    const ratio = maxStep / distance;
    this.position = {
      x: this.position.x + dx * ratio,
      y: this.position.y + dy * ratio,
    };

    return false;
  }

  // -------------------------------------------------------------------------
  // Public API — read state
  // -------------------------------------------------------------------------

  /**
   * Returns a copy of the current position in logical CSS pixels.
   *
   * Returns a new object each call so callers cannot mutate internal state.
   */
  getPosition(): Position {
    return { x: this.position.x, y: this.position.y };
  }

  /**
   * Returns `true` while the controller is moving toward an active target.
   *
   * `false` when stopped, when `setPosition()` was called without `moveTo()`,
   * or when the target has already been reached.
   */
  isMoving(): boolean {
    return this.target !== null && !this.arrived;
  }

  /**
   * Returns `true` once the character has reached the most recent `moveTo`
   * target.
   *
   * Resets to `false` when `moveTo()`, `setPosition()`, or `stop()` is
   * called.
   */
  hasArrived(): boolean {
    return this.arrived;
  }

  /**
   * Halts movement at the current position.
   *
   * Clears the active target. `isMoving()` returns `false` and
   * `hasArrived()` returns `false` until `moveTo()` is called again.
   */
  stop(): void {
    this.target = null;
    this.arrived = false;
  }
}
