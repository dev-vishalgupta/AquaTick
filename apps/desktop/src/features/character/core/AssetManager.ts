/**
 * @file AssetManager.ts
 *
 * Loads, validates, and caches character assets from the public directory.
 *
 * # Responsibilities (this file)
 *   - Fetch `character.json` for a given characterId.
 *   - Load `spritesheet.png` as an HTMLImageElement.
 *   - Cache fully loaded assets in memory by characterId.
 *   - Prevent duplicate in-flight fetches for the same characterId.
 *   - Expose typed, read-only access to cached assets.
 *
 * # Non-responsibilities (enforced)
 *   - No drawing or rendering of any kind.
 *   - No animation or movement logic.
 *   - No React dependencies.
 *   - No Tauri dependencies.
 *   - No character state machine knowledge.
 *
 * # Asset directory convention
 *   Character assets are resolved relative to the Vite public directory:
 *
 *     /characters/{characterId}/character.json
 *     (textureUrl inside the JSON is also public-relative)
 */

import type { CharacterMetadata } from "../types";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * The fully loaded assets for a single character, ready for the RenderEngine.
 */
export interface CharacterAssets {
  /** Parsed and validated character.json contents. */
  readonly metadata: CharacterMetadata;
  /** Fully decoded spritesheet image, ready for drawImage(). */
  readonly spritesheet: HTMLImageElement;
}

// ---------------------------------------------------------------------------
// AssetManager
// ---------------------------------------------------------------------------

/**
 * AssetManager
 *
 * Singleton-safe, framework-agnostic asset loader and cache.
 *
 * Call `load(characterId)` to fetch a character's JSON and spritesheet.
 * Subsequent calls for the same `characterId` return cached data immediately
 * without issuing new network requests.
 */
export class AssetManager {
  /** Resolved assets, keyed by characterId. */
  private readonly cache = new Map<string, CharacterAssets>();

  /**
   * In-flight load Promises, keyed by characterId.
   *
   * Prevents duplicate concurrent fetches when multiple callers request the
   * same characterId before the first request completes.
   */
  private readonly pending = new Map<string, Promise<CharacterAssets>>();

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Returns fully loaded assets for the given character.
   *
   * - If already cached: resolves synchronously with the cached value.
   * - If already loading: returns the existing in-flight Promise.
   * - Otherwise: begins loading JSON + spritesheet and caches the result.
   *
   * @param characterId  Must match the `characterId` field in character.json.
   * @throws             If the JSON fetch fails, the image fails to load, or
   *                     the characterId in the JSON does not match the argument.
   */
  async load(characterId: string): Promise<CharacterAssets> {
    const cached = this.cache.get(characterId);
    if (cached !== undefined) return cached;

    const inFlight = this.pending.get(characterId);
    if (inFlight !== undefined) return inFlight;

    const loadPromise = this.doLoad(characterId);
    this.pending.set(characterId, loadPromise);

    try {
      const assets = await loadPromise;
      this.cache.set(characterId, assets);
      return assets;
    } finally {
      // Always remove the pending entry whether the load succeeded or failed.
      this.pending.delete(characterId);
    }
  }

  /**
   * Returns cached assets for a character without triggering a network request.
   *
   * Returns `undefined` if the character has not been loaded yet.
   * Use this in hot paths (e.g. render loops) after a prior `load()` call.
   */
  getIfLoaded(characterId: string): CharacterAssets | undefined {
    return this.cache.get(characterId);
  }

  /**
   * Returns `true` if assets for the given characterId are fully cached.
   */
  isLoaded(characterId: string): boolean {
    return this.cache.has(characterId);
  }

  /**
   * Removes all cached assets from memory.
   *
   * In-flight loads are unaffected; they will still resolve, but their results
   * will NOT be re-added to the cache once cleared.
   */
  clearCache(): void {
    this.cache.clear();
  }

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /**
   * Performs the actual fetch + image load sequence.
   * Always called with a unique characterId (deduplication is handled above).
   */
  private async doLoad(characterId: string): Promise<CharacterAssets> {
    // -- 1. Fetch and parse character.json ------------------------------------

    const metadataUrl = `/characters/${characterId}/character.json`;

    const response = await fetch(metadataUrl);
    if (!response.ok) {
      throw new Error(
        `AssetManager: Failed to fetch metadata for "${characterId}". ` +
          `HTTP ${response.status} — ${metadataUrl}`,
      );
    }

    // The JSON is cast to CharacterMetadata; the validation below guards
    // against malformed files.
    const metadata = (await response.json()) as CharacterMetadata;

    // -- 2. Validate the loaded metadata --------------------------------------

    if (metadata.characterId !== characterId) {
      throw new Error(
        `AssetManager: characterId mismatch in "${metadataUrl}". ` +
          `Expected "${characterId}", found "${metadata.characterId}".`,
      );
    }

    if (
      typeof metadata.meta?.frameWidth !== "number" ||
      typeof metadata.meta?.frameHeight !== "number" ||
      typeof metadata.meta?.textureUrl !== "string" ||
      typeof metadata.meta?.totalRows !== "number" ||
      typeof metadata.meta?.sourceScale !== "number"
    ) {
      throw new Error(
        `AssetManager: Invalid or incomplete metadata.meta block in ` +
          `"${metadataUrl}".`,
      );
    }

    if (
      !metadata.animations ||
      Object.keys(metadata.animations).length === 0
    ) {
      throw new Error(
        `AssetManager: No animations defined in "${metadataUrl}".`,
      );
    }

    // -- 3. Load the spritesheet image ----------------------------------------

    const spritesheet = await loadImage(metadata.meta.textureUrl);

    return { metadata, spritesheet };
  }
}

// ---------------------------------------------------------------------------
// Module-private utility
// ---------------------------------------------------------------------------

/**
 * Resolves when the image at `url` is fully decoded and ready for drawImage().
 * Rejects with a descriptive error if the image cannot be loaded.
 */
function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () =>
      reject(new Error(`AssetManager: Failed to load image: "${url}"`));
    img.src = url;
  });
}
