/**
 * @file CharacterController.ts
 *
 * Framework-agnostic controller implementing the CharacterSystemAPI.
 * Coordinates the AssetManager, RenderEngine, AnimationController,
 * MovementController, and CharacterStateMachine.
 */

import type {
  CharacterSystemAPI,
  CharacterScale,
  PlayOptions,
} from "../types";
import { AssetManager } from "./AssetManager";
import { RenderEngine } from "./RenderEngine";
import { AnimationController } from "./AnimationController";
import { MovementController } from "./MovementController";
import { CharacterStateMachine } from "./CharacterStateMachine";
import { DEFAULT_WALK_SPEED_PX_PER_SEC } from "../constants";

// ---------------------------------------------------------------------------
// Scale mapping multipliers
// ---------------------------------------------------------------------------

const SCALE_MULTIPLIERS: Record<CharacterScale, number> = {
  small: 0.75,
  medium: 1.0,
  large: 1.5,
};

// ---------------------------------------------------------------------------
// CharacterController
// ---------------------------------------------------------------------------

export class CharacterController implements CharacterSystemAPI {
  private readonly assetManager: AssetManager;
  private readonly renderEngine: RenderEngine;
  private readonly animationController: AnimationController;
  private readonly movementController: MovementController;
  private readonly stateMachine: CharacterStateMachine;

  private scaleSetting: CharacterScale = "medium";
  private activeCharacterId: string | null = null;

  // Render loop tracking
  private rafId: number | null = null;
  private lastTickTime: number = 0;

  // Promise resolvers for asynchronous operations
  private walkResolver: (() => void) | null = null;
  private animResolver: (() => void) | null = null;
  private leaveResolver: (() => void) | null = null;

  /**
   * @param canvas  The canvas element to render onto.
   */
  constructor(canvas: HTMLCanvasElement) {
    this.assetManager = new AssetManager();
    this.renderEngine = new RenderEngine(canvas);
    this.animationController = new AnimationController();
    this.movementController = new MovementController();
    this.stateMachine = new CharacterStateMachine(
      this.animationController,
      this.movementController
    );
  }

  // -------------------------------------------------------------------------
  // Public API — CharacterSystemAPI Contract
  // -------------------------------------------------------------------------

  /**
   * Makes the character visible.
   * Transitions from Hidden -> Entering.
   */
  show(): void {
    if (!this.activeCharacterId) {
      console.warn("CharacterController: Cannot show character; no character is set.");
      return;
    }

    if (this.stateMachine.getState() !== "Hidden") {
      return;
    }

    // Determine initial entry position: bottom-left of the canvas viewport
    const size = this.renderEngine.getLogicalSize();
    const defaultY = size.height - 120; // 120px offset from bottom
    this.movementController.setPosition(100, defaultY);

    this.stateMachine.transitionTo("Entering");
    this.startLoop();
  }

  /**
   * Instantly cuts the character offscreen, stopping loops and clearing canvas.
   */
  hide(): void {
    if (this.stateMachine.getState() === "Hidden") {
      return;
    }

    this.stateMachine.transitionTo("Hidden");
    this.cleanupResolvers();
    this.stopLoop();
    this.renderEngine.clear();
  }

  /**
   * Walk interpolation to logical target coordinates.
   */
  walkTo(x: number, y: number, speed?: number): Promise<void> {
    if (!this.stateMachine.canTransitionTo("Walking")) {
      return Promise.reject(
        new Error(
          `CharacterController: Cannot walkTo while in state "${this.stateMachine.getState()}".`
        )
      );
    }

    // Resolve any previous in-progress walk to prevent hanging Promises
    if (this.walkResolver) {
      this.walkResolver();
      this.walkResolver = null;
    }

    this.stateMachine.transitionTo("Walking", {
      targetX: x,
      targetY: y,
      speed: speed ?? DEFAULT_WALK_SPEED_PX_PER_SEC,
    });

    return new Promise<void>((resolve) => {
      this.walkResolver = resolve;
    });
  }

  /**
   * Plays the designated animation.
   * One-shots resolve when finished; looping animations resolve immediately.
   */
  play(animationName: string, options?: PlayOptions): Promise<void> {
    let targetState: typeof this.stateMachine extends { transitionTo(state: infer S, ...args: any[]): void } ? S : any;

    if (animationName === "pickBottle") {
      targetState = "PickBottle";
    } else if (animationName === "drinkLoop") {
      targetState = "DrinkLoop";
    } else if (animationName === "putBottleDown") {
      targetState = "PutBottleDown";
    } else if (animationName === "walk") {
      targetState = "Walking";
    } else {
      return Promise.reject(
        new Error(`CharacterController: Unknown animation name "${animationName}".`)
      );
    }

    if (!this.stateMachine.canTransitionTo(targetState)) {
      return Promise.reject(
        new Error(
          `CharacterController: Cannot play "${animationName}" from state "${this.stateMachine.getState()}".`
        )
      );
    }

    // Resolve any previous animation Promise
    if (this.animResolver) {
      this.animResolver();
      this.animResolver = null;
    }

    this.stateMachine.transitionTo(targetState, { playOptions: options });

    const isLooping = options?.loop ?? this.animationController.getDefinition()?.loop ?? false;
    if (isLooping) {
      return Promise.resolve();
    }

    return new Promise<void>((resolve) => {
      this.animResolver = resolve;
    });
  }

  /**
   * Leaving sequence: walk off-screen right and hide.
   */
  leave(): Promise<void> {
    if (!this.stateMachine.canTransitionTo("Leaving")) {
      return Promise.reject(
        new Error(
          `CharacterController: Cannot leave from state "${this.stateMachine.getState()}".`
        )
      );
    }

    // Resolve previous active operations
    this.cleanupResolvers();

    const size = this.renderEngine.getLogicalSize();
    const currentPos = this.movementController.getPosition();
    const targetX = size.width + 150; // 150px offscreen right

    this.stateMachine.transitionTo("Leaving", {
      targetX,
      targetY: currentPos.y,
      speed: DEFAULT_WALK_SPEED_PX_PER_SEC,
    });

    return new Promise<void>((resolve) => {
      this.leaveResolver = resolve;
    });
  }

  /**
   * Set visual size multiplier scale.
   */
  setScale(scale: CharacterScale): void {
    this.scaleSetting = scale;
  }

  /**
   * Eager load assets via AssetManager and update metadata.
   */
  async setCharacter(characterId: string): Promise<void> {
    const assets = await this.assetManager.load(characterId);
    this.activeCharacterId = characterId;
    this.stateMachine.setMetadata(assets.metadata);
  }

  // -------------------------------------------------------------------------
  // Private render loop & tick logic
  // -------------------------------------------------------------------------

  private startLoop(): void {
    if (this.rafId !== null) return;
    this.lastTickTime = 0;
    this.rafId = requestAnimationFrame(this.tickLoop);
  }

  private stopLoop(): void {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    this.lastTickTime = 0;
  }

  private tickLoop = (timestamp: number): void => {
    if (this.stateMachine.getState() === "Hidden") {
      this.stopLoop();
      return;
    }

    if (this.lastTickTime === 0) {
      this.lastTickTime = timestamp;
    }

    let deltaMs = timestamp - this.lastTickTime;
    this.lastTickTime = timestamp;

    // Clamp delta time to avoid large jumps when tab is unfocused
    if (deltaMs > 100) {
      deltaMs = 100;
    }

    // Tick Animation
    const animCompleted = this.animationController.tick(deltaMs);
    if (animCompleted && this.animResolver) {
      const resolve = this.animResolver;
      this.animResolver = null;
      resolve();
    }

    // Tick Movement
    const movementCompleted = this.movementController.tick(deltaMs);
    if (movementCompleted) {
      if (this.stateMachine.getState() === "Walking" && this.walkResolver) {
        const resolve = this.walkResolver;
        this.walkResolver = null;
        resolve();
      } else if (this.stateMachine.getState() === "Leaving") {
        this.stateMachine.transitionTo("Hidden");
        this.renderEngine.clear();
        this.stopLoop();
        if (this.leaveResolver) {
          const resolve = this.leaveResolver;
          this.leaveResolver = null;
          resolve();
        }
        return; // Loop stopped, exit early
      }
    }

    // Clear and draw active frame
    const currentState = this.stateMachine.getState();
    if (currentState !== "Hidden") {
      this.renderEngine.clear();

      const assets = this.assetManager.getIfLoaded(this.activeCharacterId!);
      const animDef = this.animationController.getDefinition();

      if (assets && animDef) {
        const pos = this.movementController.getPosition();
        const scale = SCALE_MULTIPLIERS[this.scaleSetting];
        const frameIdx = this.animationController.getFrameIndex();

        this.renderEngine.drawFrame(
          assets.spritesheet,
          assets.metadata.meta,
          animDef,
          frameIdx,
          pos.x,
          pos.y,
          scale
        );
      }
    }

    this.rafId = requestAnimationFrame(this.tickLoop);
  };

  private cleanupResolvers(): void {
    if (this.walkResolver) {
      this.walkResolver();
      this.walkResolver = null;
    }
    if (this.animResolver) {
      this.animResolver();
      this.animResolver = null;
    }
    if (this.leaveResolver) {
      this.leaveResolver();
      this.leaveResolver = null;
    }
  }
}
