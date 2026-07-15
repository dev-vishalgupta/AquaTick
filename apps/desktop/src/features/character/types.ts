/**
 * @file types.ts
 *
 * Domain models and public interfaces for the Character System.
 *
 * Architectural rules enforced by this module:
 *   1. No React imports — this file is framework-agnostic.
 *   2. No Tauri imports — the Character System never talks to the backend.
 *   3. No side effects — pure type and interface declarations only.
 *
 * Every other Character System module must import its shared vocabulary from
 * here rather than re-declaring local variants.
 */

import type { CHARACTER_SCALES } from "./constants";

// ---------------------------------------------------------------------------
// Visual State Machine
// ---------------------------------------------------------------------------

/**
 * The exhaustive set of visual states the character can occupy.
 *
 * Mapping to architecture document (Section 8):
 *
 *   Hidden          → character is off-screen; render loop is suspended.
 *   Entering        → character has become visible at the entry origin.
 *   Walking         → character is moving toward a target coordinate.
 *   PickBottle      → one-shot pick-up animation is playing.
 *   DrinkLoop       → looping drinking animation; awaits an external command.
 *   PutBottleDown   → one-shot put-down animation is playing.
 *   Leaving         → character is walking off-screen toward the exit origin.
 *
 * Valid transitions:
 *   Hidden       → Entering        (via show())
 *   Entering     → Walking         (via walkTo())
 *   Walking      → PickBottle      (via play('pickBottle'))
 *   PickBottle   → DrinkLoop       (via play('drinkLoop'))
 *   DrinkLoop    → PutBottleDown   (via play('putBottleDown'))
 *   PutBottleDown→ Leaving         (via leave())
 *   Leaving      → Hidden          (automatically, when off-screen)
 *   <any Visible>→ Hidden          (via hide() — instant cut)
 */
export type CharacterVisualState =
  | "Hidden"
  | "Entering"
  | "Walking"
  | "PickBottle"
  | "DrinkLoop"
  | "PutBottleDown"
  | "Leaving";

/** The subset of states where the character is drawn on screen. */
export type CharacterVisibleState = Exclude<CharacterVisualState, "Hidden">;

// ---------------------------------------------------------------------------
// Scale
// ---------------------------------------------------------------------------

/**
 * User-selectable character viewport scale.
 *
 * Applied once at mount time. Fluid runtime scale changes are a deferred
 * feature (see Architecture §15).
 */
export type CharacterScale = (typeof CHARACTER_SCALES)[number];

// ---------------------------------------------------------------------------
// Asset Metadata
// ---------------------------------------------------------------------------

/**
 * Configuration for a single named animation strip within the sprite sheet.
 *
 * Maps to the `animations.<key>` shape in `character.json` (Architecture §11).
 */
export interface AnimationDefinition {
  /** Zero-based row index within the sprite sheet. */
  readonly row: number;
  /** Total number of frames in this animation strip. */
  readonly frameCount: number;
  /** Target playback rate in frames per second. */
  readonly fps: number;
  /** When true the animation cycles indefinitely; when false it plays once. */
  readonly loop: boolean;
}

/**
 * Physical and layout metadata for the sprite sheet texture.
 *
 * Maps to the `meta` block of `character.json`.
 */
export interface SpritesheetMeta {
  /** Path (relative to the public asset directory) to the sprite sheet PNG. */
  readonly textureUrl: string;
  /** Pixel width of a single frame. */
  readonly frameWidth: number;
  /** Pixel height of a single frame. */
  readonly frameHeight: number;
  /** Total number of animation rows present in the sheet. */
  readonly totalRows: number;
  /**
   * Original art scale factor.
   *
   * A value of `2.0` indicates the source art was drawn at 2× resolution;
   * the Render Engine accounts for this when mapping frames to display pixels.
   */
  readonly sourceScale: number;
}

/**
 * Complete metadata descriptor for one character.
 *
 * This is the top-level shape of every `character.json` file.
 */
export interface CharacterMetadata {
  /** Unique identifier that matches the `characterId` used in API calls. */
  readonly characterId: string;
  /** Display-friendly character name. */
  readonly name: string;
  /** Sprite sheet physical layout. */
  readonly meta: SpritesheetMeta;
  /** Named animation strips keyed by animation identifier (e.g. "walk"). */
  readonly animations: Readonly<Record<string, AnimationDefinition>>;
}

// ---------------------------------------------------------------------------
// Coordinate types
// ---------------------------------------------------------------------------

/**
 * A two-dimensional position in canvas-local pixels.
 *
 * All coordinates passed through the public API are in logical pixels.
 * The Render Engine is responsible for translating them to physical pixels
 * when DPI scaling is applied.
 */
export interface Position {
  readonly x: number;
  readonly y: number;
}

// ---------------------------------------------------------------------------
// Public API Options
// ---------------------------------------------------------------------------

/**
 * Options that refine the behaviour of a `play()` call.
 *
 * All fields are optional; any omitted value falls back to the definition
 * supplied by the active `AnimationDefinition`.
 */
export interface PlayOptions {
  /**
   * Override the loop flag from the animation definition.
   *
   * When set to `true` the animation cycles until another command is received.
   * When set to `false` it plays once and the returned Promise resolves.
   */
  loop?: boolean;
  /**
   * Override the frames-per-second from the animation definition.
   */
  fps?: number;
  /**
   * Callback fired every time the frame index advances.
   *
   * Intended for audio-sync hooks (Architecture §12 — Audio Sync Triggers).
   * The Character System itself never plays sound.
   */
  onFrame?: (frameIndex: number) => void;
}

// ---------------------------------------------------------------------------
// Public API Contract
// ---------------------------------------------------------------------------

/**
 * The complete public interface of the Character System.
 *
 * This interface is the only surface that external systems (Frontend Event
 * Coordinator, developer test utilities) may use to drive the character.
 *
 * Design rules (Architecture §7, §14):
 *   • Each method has exactly one responsibility.
 *   • `show()` makes the character visible; it does NOT imply movement.
 *   • `walkTo()` moves the character; it does NOT make the character visible.
 *   • Methods that involve time-consuming operations return a Promise that
 *     resolves when the operation is fully complete.
 *   • Synchronous methods (`show`, `hide`, `setScale`) complete instantly.
 *   • The interface contains no business-domain vocabulary (sessions,
 *     reminders, schedules, databases).
 */
export interface CharacterSystemAPI {
  /**
   * Makes the character visible at its current (or default entry) position.
   *
   * Transitions the visual state from `Hidden` to `Entering`.
   * Does NOT trigger movement — call `walkTo()` afterward.
   */
  show(): void;

  /**
   * Instantly hides the character and cancels all pending animations and
   * movement.
   *
   * Transitions any visible state directly to `Hidden`.
   * The render loop suspends immediately.
   */
  hide(): void;

  /**
   * Moves the character from their current position to `(x, y)`.
   *
   * @param x      Target x coordinate in logical pixels.
   * @param y      Target y coordinate in logical pixels.
   * @param speed  Movement speed in logical pixels per second. Defaults to
   *               `DEFAULT_WALK_SPEED_PX_PER_SEC`.
   * @returns      A Promise that resolves when the destination is reached.
   */
  walkTo(x: number, y: number, speed?: number): Promise<void>;

  /**
   * Plays the named animation.
   *
   * The character loops the animation indefinitely until a subsequent command
   * is received — the Character System never owns the "waiting" decision.
   *
   * @param animationName  Key into the `animations` record of the loaded
   *                       `CharacterMetadata` (e.g. `"drinkLoop"`).
   * @param options        Optional overrides for loop, fps, and onFrame.
   * @returns              A Promise that resolves when a non-looping animation
   *                       completes its single playthrough. For looping
   *                       animations the Promise never resolves on its own;
   *                       the caller must issue the next command to proceed.
   */
  play(animationName: string, options?: PlayOptions): Promise<void>;

  /**
   * Executes the full leaving sequence.
   *
   * The character walks off-screen toward the exit origin (bottom-right) and
   * then transitions to `Hidden`.
   *
   * @returns A Promise that resolves when the character is completely
   *          off-screen and the visual state is `Hidden`.
   */
  leave(): Promise<void>;

  /**
   * Updates the visual scale of the character viewport.
   *
   * Scale is applied immediately and affects all subsequent rendering.
   * The character's logical coordinates remain unchanged.
   */
  setScale(scale: CharacterScale): void;

  /**
   * Swaps the active character to the one identified by `characterId`.
   *
   * Triggers the Asset Manager to load the corresponding `character.json`
   * and sprite sheet. The character is ready for commands once the returned
   * Promise resolves.
   *
   * @param characterId  Must match the `characterId` field in `character.json`.
   * @returns            A Promise that resolves when assets are fully loaded.
   */
  setCharacter(characterId: string): Promise<void>;
}
