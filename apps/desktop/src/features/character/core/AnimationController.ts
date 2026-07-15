/**
 * @file AnimationController.ts
 *
 * Time-based frame calculator for sprite sheet animations.
 *
 * # Responsibilities (this file)
 *   - Accept an animation definition and compute the current frame index
 *     from cumulative elapsed time.
 *   - Handle configurable FPS (frames per second).
 *   - Distinguish looping animations (cycle indefinitely) from one-shot
 *     animations (play once then stop).
 *   - Detect and signal animation completion for one-shot playbacks.
 *   - Fire the `onFrame` callback when the frame index advances (for
 *     external audio-sync hooks — Architecture §12).
 *
 * # Non-responsibilities (enforced)
 *   - No rendering or drawing of any kind.
 *   - No requestAnimationFrame or timer ownership. The caller (CharacterController,
 *     Phase 5A.6) drives ticks at its own cadence.
 *   - No state machine transitions.
 *   - No React or Tauri dependencies.
 *
 * # Tick model
 *   The controller is driven externally. On each iteration of the caller's
 *   render loop, the caller passes the elapsed time since the last iteration
 *   (deltaMs) to `tick()`. The controller advances internal time and returns
 *   the updated frame index through `getFrameIndex()`.
 *
 *   Frame index formula:
 *     rawFrame  = floor(elapsedMs × fps / 1000)
 *     looping:  frameIndex = rawFrame % frameCount
 *     one-shot: frameIndex = min(rawFrame, frameCount − 1)
 *               complete when rawFrame ≥ frameCount
 */

import type { AnimationDefinition, PlayOptions } from "../types";

// ---------------------------------------------------------------------------
// AnimationController
// ---------------------------------------------------------------------------

/**
 * AnimationController
 *
 * Computes frame indices for sprite sheet playback.
 * One instance per animation slot (the CharacterController owns the instance).
 *
 * @example
 * ```ts
 * const anim = new AnimationController();
 * anim.play(metadata.animations.walk);
 *
 * // Inside the render loop:
 * const justFinished = anim.tick(deltaMs);
 * engine.drawFrame(sheet, meta, animDef, anim.getFrameIndex(), x, y, scale);
 * ```
 */
export class AnimationController {
  // -------------------------------------------------------------------------
  // Private state
  // -------------------------------------------------------------------------

  /** The active animation strip definition. Null when stopped. */
  private animDef: AnimationDefinition | null = null;

  /** Effective loop flag — merged from definition + PlayOptions override. */
  private looping: boolean = false;

  /** Effective FPS — merged from definition + PlayOptions override. */
  private effectiveFps: number = 0;

  /** Optional per-frame callback for audio-sync (Architecture §12). */
  private onFrameCallback: ((frameIndex: number) => void) | undefined =
    undefined;

  /** Cumulative elapsed milliseconds since the last `play()` call. */
  private elapsedMs: number = 0;

  /** Current frame index (0-based). Read via `getFrameIndex()`. */
  private currentFrameIndex: number = 0;

  /** True after a one-shot animation has played its final frame. */
  private complete: boolean = false;

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Begins (or restarts) playback of the given animation.
   *
   * Always resets elapsed time to zero, so calling `play()` mid-animation
   * starts the new animation from its first frame.
   *
   * @param definition  Animation strip configuration from CharacterMetadata.
   * @param options     Optional overrides for `loop`, `fps`, and `onFrame`.
   *                    Missing fields fall back to the definition values.
   */
  play(definition: AnimationDefinition, options?: PlayOptions): void {
    this.animDef = definition;
    this.looping = options?.loop ?? definition.loop;
    this.effectiveFps = options?.fps ?? definition.fps;
    this.onFrameCallback = options?.onFrame;
    this.elapsedMs = 0;
    this.currentFrameIndex = 0;
    this.complete = false;
  }

  /**
   * Advances the animation by `deltaMs` milliseconds.
   *
   * Must be called on every iteration of the render loop while an animation
   * is active.
   *
   * @param deltaMs  Milliseconds elapsed since the previous `tick()` call.
   *                 Must be ≥ 0. Negative values are treated as zero.
   * @returns        `true` exactly once — on the tick when a one-shot animation
   *                 plays its final frame. Returns `false` for all other ticks,
   *                 including all ticks of looping animations. Use this signal
   *                 to chain animations without polling `isComplete()`.
   */
  tick(deltaMs: number): boolean {
    if (this.animDef === null || this.complete) return false;

    const safeDelta = Math.max(0, deltaMs);
    this.elapsedMs += safeDelta;

    const prevFrame = this.currentFrameIndex;
    const msPerFrame = 1000 / this.effectiveFps;

    // Guard against zero/negative FPS (degenerate definition): freeze on
    // frame 0 rather than dividing by zero or producing NaN.
    if (!isFinite(msPerFrame) || msPerFrame <= 0) return false;

    const rawFrame = Math.floor(this.elapsedMs / msPerFrame);

    let justCompleted = false;

    if (this.looping) {
      // Looping: wrap frame index indefinitely.
      this.currentFrameIndex = rawFrame % this.animDef.frameCount;
    } else {
      // One-shot: clamp to last frame and signal completion once.
      if (rawFrame >= this.animDef.frameCount) {
        this.currentFrameIndex = this.animDef.frameCount - 1;
        if (!this.complete) {
          this.complete = true;
          justCompleted = true;
        }
      } else {
        this.currentFrameIndex = rawFrame;
      }
    }

    // Fire onFrame only when the frame index actually changes.
    // This mirrors the natural cadence of the animation (one event per
    // frame advance) rather than firing on every tick.
    if (this.currentFrameIndex !== prevFrame && this.onFrameCallback) {
      this.onFrameCallback(this.currentFrameIndex);
    }

    return justCompleted;
  }

  /**
   * Returns the current frame index to pass to `RenderEngine.drawFrame()`.
   *
   * Returns 0 when no animation is playing.
   */
  getFrameIndex(): number {
    return this.currentFrameIndex;
  }

  /**
   * Returns `true` when a one-shot animation has played its final frame.
   *
   * Always `false` for looping animations (they never self-complete).
   * Resets to `false` on the next `play()` call.
   */
  isComplete(): boolean {
    return this.complete;
  }

  /**
   * Returns `true` while an animation is loaded and not yet complete.
   *
   * `false` when stopped, or when a one-shot animation has finished.
   */
  isPlaying(): boolean {
    return this.animDef !== null && !this.complete;
  }

  /**
   * Returns the active animation definition, or `null` if stopped.
   *
   * Provided so the CharacterController can pass the definition directly
   * to `RenderEngine.drawFrame()` without storing it separately.
   */
  getDefinition(): AnimationDefinition | null {
    return this.animDef;
  }

  /**
   * Halts the current animation and resets all internal state.
   *
   * After calling `stop()`, `isPlaying()` returns `false` and
   * `getFrameIndex()` returns 0 until `play()` is called again.
   */
  stop(): void {
    this.animDef = null;
    this.looping = false;
    this.effectiveFps = 0;
    this.onFrameCallback = undefined;
    this.elapsedMs = 0;
    this.currentFrameIndex = 0;
    this.complete = false;
  }
}
