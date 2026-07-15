/**
 * @file constants.ts
 *
 * Shared constants for the Character System.
 *
 * Architectural note: This module is pure data. It has no dependencies on
 * React, Tauri, or any rendering layer. It exists solely to document and
 * centralise the discrete values that the rest of the Character System uses.
 */

// ---------------------------------------------------------------------------
// Character Scale
// ---------------------------------------------------------------------------

/**
 * The set of supported character viewport scales.
 *
 * Scale is a single setting applied at mount time and does not change during
 * rendering. Runtime scale changes are a deferred feature.
 */
export const CHARACTER_SCALES = ["small", "medium", "large"] as const;

// ---------------------------------------------------------------------------
// Default animation identifiers
//
// These names correspond to keys inside the `animations` record of
// CharacterMetadata.  They are kept here so that call-sites can reference
// constants rather than raw strings.
// ---------------------------------------------------------------------------

/** Standard animation identifier keys shipped with the MVP character. */
export const ANIMATION_KEYS = {
  WALK: "walk",
  PICK_BOTTLE: "pickBottle",
  DRINK_LOOP: "drinkLoop",
  PUT_BOTTLE_DOWN: "putBottleDown",
} as const;

// ---------------------------------------------------------------------------
// Default movement speed (pixels per second at medium scale)
// ---------------------------------------------------------------------------

/**
 * Fallback movement speed used when `walkTo()` is called without an explicit
 * `speed` argument.
 */
export const DEFAULT_WALK_SPEED_PX_PER_SEC = 120;
