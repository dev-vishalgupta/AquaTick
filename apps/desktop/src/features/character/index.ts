/**
 * @file index.ts
 *
 * Character System — public barrel export.
 *
 * All external consumers (pages, Event Coordinator) must import from this
 * file rather than reaching into individual sub-modules. This keeps the
 * internal module structure free to change without breaking call-sites.
 *
 * Exports grow phase by phase:
 *   Phase 5A.1  → types and constants.
 *   Phase 5A.2  → CharacterCanvas host component.
 *   Phase 5A.3  → AssetManager, RenderEngine.
 *   Phase 5A.4  → AnimationController, MovementController.
 *   Phase 5A.5  → CharacterStateMachine.
 *   Phase 5A.6  → CharacterController / CharacterSystemAPI implementation.  ← current phase
 */

// -- Phase 5A.2: React Host --------------------------------------------------

export { default as CharacterCanvas } from "./components/CharacterCanvas";
export type { CharacterCanvasHandle } from "./components/CharacterCanvas";

// -- Phase 5A.3: Core rendering layer ----------------------------------------

export { AssetManager } from "./core/AssetManager";
export type { CharacterAssets } from "./core/AssetManager";

export { RenderEngine } from "./core/RenderEngine";

// -- Phase 5A.4: Core controllers --------------------------------------------

export { AnimationController } from "./core/AnimationController";
export { MovementController } from "./core/MovementController";

// -- Phase 5A.5: Character State Machine --------------------------------------

export { CharacterStateMachine } from "./core/CharacterStateMachine";
export type { TransitionContext } from "./core/CharacterStateMachine";

// -- Phase 5A.6: Character Controller -----------------------------------------

export { CharacterController } from "./core/CharacterController";

// -- Phase 5A.1: Domain types (re-exported for consumer convenience) ----------

export type {
  CharacterSystemAPI,
  CharacterVisualState,
  CharacterVisibleState,
  CharacterScale,
  CharacterMetadata,
  SpritesheetMeta,
  AnimationDefinition,
  Position,
  PlayOptions,
} from "./types";
