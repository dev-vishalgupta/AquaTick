/**
 * @file RenderEngine.ts
 *
 * Owns all Canvas 2D rendering operations for the Character System.
 *
 * # Responsibilities (this file)
 *   - Accept an HTMLCanvasElement and obtain its 2D context.
 *   - Clear the canvas pixel buffer.
 *   - Draw a single frame from a sprite sheet at a given logical position.
 *   - Handle DPI scaling internally so all public coordinate parameters
 *     are expressed in logical CSS pixels.
 *
 * # Non-responsibilities (enforced)
 *   - No requestAnimationFrame or render loop of any kind.
 *   - No animation sequencing or frame advancement.
 *   - No movement interpolation.
 *   - No React dependencies.
 *   - No character state machine knowledge.
 *   - No asset loading (that belongs to AssetManager).
 *
 * # DPI scaling contract
 *   Phase 5A.2 (CharacterCanvas) sets the canvas pixel buffer to physical
 *   pixels:
 *
 *     canvas.width  = cssWidth  × devicePixelRatio
 *     canvas.height = cssHeight × devicePixelRatio
 *
 *   RenderEngine mirrors this by applying `ctx.setTransform(dpr, 0, 0, dpr,
 *   0, 0)` around every draw call. This maps logical CSS pixels → physical
 *   buffer pixels, ensuring crisp output at any DPI without the caller
 *   needing to know the current ratio.
 *
 *   The DPR is read fresh from `window.devicePixelRatio` on each draw call
 *   so that OS-level DPI changes (e.g. moving the app window to a different
 *   monitor) are automatically handled.
 */

import type { AnimationDefinition, SpritesheetMeta } from "../types";

// ---------------------------------------------------------------------------
// RenderEngine
// ---------------------------------------------------------------------------

/**
 * RenderEngine
 *
 * Low-level canvas drawing API for the Character System.
 * One instance is created per canvas element by the CharacterController
 * (Phase 5A.6).
 *
 * @example
 * ```ts
 * const engine = new RenderEngine(canvas);
 * engine.clear();
 * engine.drawFrame(spritesheet, meta, animDef, 0, centerX, centerY, 1.0);
 * ```
 */
export class RenderEngine {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;

  /**
   * @param canvas  The HTMLCanvasElement managed by CharacterCanvas (Phase 5A.2).
   * @throws        If the browser cannot provide a 2D rendering context
   *                (e.g. the canvas is already owned by a WebGL context).
   */
  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (ctx === null) {
      throw new Error(
        "RenderEngine: Failed to obtain CanvasRenderingContext2D. " +
          "Ensure the canvas is not already bound to a WebGL context.",
      );
    }
    this.canvas = canvas;
    this.ctx = ctx;
  }

  // -------------------------------------------------------------------------
  // Public drawing API
  // -------------------------------------------------------------------------

  /**
   * Erases the entire canvas pixel buffer.
   *
   * Uses physical pixel dimensions (`canvas.width`, `canvas.height`) directly,
   * bypassing the DPI transform, so the full buffer is always cleared
   * regardless of current devicePixelRatio.
   *
   * Should be called at the start of every render cycle before any draw calls.
   */
  clear(): void {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  /**
   * Draws one frame from a sprite sheet at the specified logical position.
   *
   * The sprite is centered on `(x, y)`.
   *
   * # Source rectangle
   * Derived from the animation definition and frame index:
   *   - x_src = frameIndex × meta.frameWidth
   *   - y_src = animDef.row × meta.frameHeight
   *   - width  = meta.frameWidth
   *   - height = meta.frameHeight
   *
   * # Destination size
   * Corrected for the original art scale factor:
   *   displayW = (meta.frameWidth  / meta.sourceScale) × scale
   *   displayH = (meta.frameHeight / meta.sourceScale) × scale
   *
   * A `sourceScale` of 2.0 means the sprite was drawn at 2× resolution.
   * Dividing by sourceScale maps it back to a 1:1 physical-pixel size before
   * the caller's `scale` multiplier is applied.
   *
   * @param spritesheet  Fully loaded spritesheet image (from AssetManager).
   * @param meta         Sprite sheet physical layout metadata.
   * @param animDef      Animation strip definition (provides row index).
   * @param frameIndex   Zero-based frame index within the animation strip.
   *                     Clamped to [0, animDef.frameCount - 1].
   * @param x            Logical X coordinate of the sprite center (CSS pixels).
   * @param y            Logical Y coordinate of the sprite center (CSS pixels).
   * @param scale        Display scale multiplier applied after sourceScale
   *                     correction. Use 1.0 for native size.
   */
  drawFrame(
    spritesheet: HTMLImageElement,
    meta: SpritesheetMeta,
    animDef: AnimationDefinition,
    frameIndex: number,
    x: number,
    y: number,
    scale: number,
  ): void {
    const dpr = window.devicePixelRatio || 1;

    // Clamp frameIndex to valid range to prevent drawing outside the sheet.
    const clampedFrame = Math.max(
      0,
      Math.min(Math.floor(frameIndex), animDef.frameCount - 1),
    );

    // -- Source rectangle (pixels in the sprite sheet) -----------------------

    const sx = clampedFrame * meta.frameWidth;
    const sy = animDef.row * meta.frameHeight;
    const sw = meta.frameWidth;
    const sh = meta.frameHeight;

    // -- Destination rectangle (logical CSS pixels) --------------------------

    // Divide by sourceScale to normalise art drawn at a higher base resolution,
    // then multiply by the caller's scale factor.
    const displayW = (meta.frameWidth / meta.sourceScale) * scale;
    const displayH = (meta.frameHeight / meta.sourceScale) * scale;

    // Center the sprite on (x, y).
    const dx = x - displayW / 2;
    const dy = y - displayH / 2;

    // -- Draw with DPI-aware transform ----------------------------------------

    // ctx.setTransform sets an absolute (non-cumulative) matrix. This is safer
    // than ctx.scale() which would multiply with any existing transform.
    // After restore(), the context returns to whatever state existed before.
    this.ctx.save();
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.drawImage(spritesheet, sx, sy, sw, sh, dx, dy, displayW, displayH);
    this.ctx.restore();
  }

  // -------------------------------------------------------------------------
  // Diagnostics
  // -------------------------------------------------------------------------

  /**
   * Returns the logical (CSS pixel) dimensions of the canvas.
   *
   * Computed by dividing the physical buffer dimensions by devicePixelRatio.
   * Useful for computing default center coordinates without importing React
   * layout utilities.
   */
  getLogicalSize(): { width: number; height: number } {
    const dpr = window.devicePixelRatio || 1;
    return {
      width: this.canvas.width / dpr,
      height: this.canvas.height / dpr,
    };
  }
}
