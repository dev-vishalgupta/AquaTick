/**
 * @file CharacterStateMachine.ts
 *
 * Deterministic state machine governing character visual transitions.
 *
 * # Responsibilities (this file)
 *   - Maintain the current visual state of the character.
 *   - Enforce approved state transitions and reject invalid ones.
 *   - Coordinate AnimationController and MovementController by configuring their
 *     states on transition (e.g. playing the correct animation clip, stopping movement).
 *   - Remain fully deterministic and framework-agnostic.
 *
 * # Non-responsibilities (enforced)
 *   - No rendering or canvas operations.
 *   - No direct timer or requestAnimationFrame tick management.
 *   - No Tauri or React dependencies.
 *   - No database or reminder business logic.
 *
 * # State Machine Definition (Architecture §8)
 *   States:
 *     - Hidden: character is offscreen and inactive.
 *     - Entering: character appears at entry position.
 *     - Walking: character is moving to a target bottle position.
 *     - PickBottle: character plays one-shot pick up animation.
 *     - DrinkLoop: character plays looping drink animation.
 *     - PutBottleDown: character plays one-shot put down animation.
 *     - Leaving: character is walking offscreen to exit position.
 *
 *   Allowed transitions are defined in ALLOWED_TRANSITIONS. Any attempt to
 *   perform an invalid transition will throw an error.
 */

import type { CharacterVisualState, CharacterMetadata, PlayOptions } from "../types";
import type { AnimationController } from "./AnimationController";
import type { MovementController } from "./MovementController";

// ---------------------------------------------------------------------------
// Transition Context
// ---------------------------------------------------------------------------

/**
 * Context parameters provided during a state transition.
 */
export interface TransitionContext {
  /** Target position for movement (required for Walking and Leaving states). */
  targetX?: number;
  /** Target position for movement (required for Walking and Leaving states). */
  targetY?: number;
  /** Movement speed. Falls back to default speed if omitted. */
  speed?: number;
  /** Custom options for animation playback (fps override, onFrame callbacks). */
  playOptions?: PlayOptions;
}

// ---------------------------------------------------------------------------
// Allowed Transitions Map
// ---------------------------------------------------------------------------

const ALLOWED_TRANSITIONS: Record<CharacterVisualState, Set<CharacterVisualState>> = {
  Hidden: new Set<CharacterVisualState>(["Hidden", "Entering"]),
  Entering: new Set<CharacterVisualState>(["Hidden", "Entering", "Walking"]),
  Walking: new Set<CharacterVisualState>(["Hidden", "Walking", "PickBottle"]),
  PickBottle: new Set<CharacterVisualState>(["Hidden", "PickBottle", "DrinkLoop"]),
  DrinkLoop: new Set<CharacterVisualState>(["Hidden", "DrinkLoop", "PutBottleDown"]),
  PutBottleDown: new Set<CharacterVisualState>(["Hidden", "PutBottleDown", "Leaving"]),
  Leaving: new Set<CharacterVisualState>(["Hidden", "Leaving"]),
};

// ---------------------------------------------------------------------------
// CharacterStateMachine
// ---------------------------------------------------------------------------

export class CharacterStateMachine {
  private currentState: CharacterVisualState = "Hidden";
  private metadata: CharacterMetadata | null = null;

  /**
   * @param animationController  Controller for frame ticking.
   * @param movementController   Controller for position interpolation.
   */
  constructor(
    private readonly animationController: AnimationController,
    private readonly movementController: MovementController,
  ) {}

  /**
   * Updates the active character metadata.
   *
   * Must be called when the character is changed to ensure the correct row index
   * and loop properties are queried from the metadata during transitions.
   */
  setMetadata(metadata: CharacterMetadata | null): void {
    this.metadata = metadata;
  }

  /**
   * Returns the current state of the machine.
   */
  getState(): CharacterVisualState {
    return this.currentState;
  }

  /**
   * Returns true if transitioning from the current state to the proposed state
   * is allowed by the visual state machine rules.
   */
  canTransitionTo(newState: CharacterVisualState): boolean {
    return ALLOWED_TRANSITIONS[this.currentState].has(newState);
  }

  /**
   * Transitions the machine to a new state and coordinates the controllers.
   *
   * @param newState  The state to transition into.
   * @param context   Optional movement targets and animation options.
   * @throws          If the transition is not allowed.
   */
  transitionTo(newState: CharacterVisualState, context?: TransitionContext): void {
    if (!this.canTransitionTo(newState)) {
      throw new Error(
        `CharacterStateMachine: Invalid transition from "${this.currentState}" to "${newState}".`
      );
    }

    const prevState = this.currentState;
    this.currentState = newState;

    // No-op if transition is to the same state and no new movement/animation is requested
    if (prevState === newState && !context) {
      return;
    }

    switch (newState) {
      case "Hidden":
        this.animationController.stop();
        this.movementController.stop();
        break;

      case "Entering":
        // Reset movement on entry; animation stops until walk starts
        this.animationController.stop();
        this.movementController.stop();
        break;

      case "Walking": {
        const walkDef = this.metadata?.animations["walk"];
        if (walkDef) {
          // Walking is always a looping animation
          this.animationController.play(walkDef, { loop: true });
        } else {
          this.animationController.stop();
        }

        if (context?.targetX !== undefined && context?.targetY !== undefined) {
          this.movementController.moveTo(context.targetX, context.targetY, context.speed);
        }
        break;
      }

      case "PickBottle": {
        const pickDef = this.metadata?.animations["pickBottle"];
        if (pickDef) {
          // Picking up the bottle is a one-shot animation
          this.animationController.play(pickDef, {
            ...context?.playOptions,
            loop: false,
          });
        }
        this.movementController.stop();
        break;
      }

      case "DrinkLoop": {
        const drinkDef = this.metadata?.animations["drinkLoop"];
        if (drinkDef) {
          // Drinking loop runs indefinitely until another state is triggered
          this.animationController.play(drinkDef, {
            ...context?.playOptions,
            loop: true,
          });
        }
        this.movementController.stop();
        break;
      }

      case "PutBottleDown": {
        const putDef = this.metadata?.animations["putBottleDown"];
        if (putDef) {
          // Putting down the bottle is a one-shot animation
          this.animationController.play(putDef, {
            ...context?.playOptions,
            loop: false,
          });
        }
        this.movementController.stop();
        break;
      }

      case "Leaving": {
        const walkDef = this.metadata?.animations["walk"];
        if (walkDef) {
          // Leaving uses the walking animation loop
          this.animationController.play(walkDef, { loop: true });
        } else {
          this.animationController.stop();
        }

        if (context?.targetX !== undefined && context?.targetY !== undefined) {
          this.movementController.moveTo(context.targetX, context.targetY, context.speed);
        }
        break;
      }
    }
  }
}
