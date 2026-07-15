/**
 * @file SoundSystem.ts
 *
 * Framework-independent Sound System implementation using HTMLAudioElement.
 */

import type { SoundId, SoundSystemAPI } from "./types";

class SoundSystemClass implements SoundSystemAPI {
  private readonly cache = new Map<SoundId, HTMLAudioElement>();
  private readonly activeLoops = new Map<SoundId, HTMLAudioElement>();
  private volumeSetting: number = 1.0;
  private isMutedSetting: boolean = false;

  // Sound IDs mapped to relative Vite assets paths
  private readonly soundPaths: Record<SoundId, string> = {
    footsteps: "/sounds/footsteps.mp3",
    pickup: "/sounds/pickup.mp3",
    drink: "/sounds/drink.mp3",
    notification: "/sounds/notification.mp3",
    click: "/sounds/click.mp3",
  };

  /**
   * Preloads a sound into the cache.
   *
   * @param soundId The identifier of the sound to preload.
   */
  preload(soundId: SoundId): Promise<void> {
    return new Promise<void>((resolve) => {
      const path = this.soundPaths[soundId];
      const audio = new Audio(path);

      // We resolve even on error to prevent blocking initialization if assets are missing
      audio.addEventListener("canplaythrough", () => resolve(), { once: true });
      audio.addEventListener(
        "error",
        () => {
          console.warn(`SoundSystem: Failed to load sound "${soundId}" from "${path}"`);
          resolve();
        },
        { once: true }
      );

      this.cache.set(soundId, audio);
    });
  }

  /**
   * Preloads all registered sound files.
   */
  async preloadAll(): Promise<void> {
    const promises = (Object.keys(this.soundPaths) as SoundId[]).map((id) =>
      this.preload(id)
    );
    await Promise.all(promises);
  }

  /**
   * Plays a sound once.
   * Supports concurrent playback by cloning the template audio node.
   */
  play(soundId: SoundId): void {
    let template = this.cache.get(soundId);
    if (!template) {
      const path = this.soundPaths[soundId];
      template = new Audio(path);
      this.cache.set(soundId, template);
    }

    try {
      const instance = template.cloneNode(true) as HTMLAudioElement;
      instance.volume = this.isMutedSetting ? 0 : this.volumeSetting;
      instance.play().catch((err) => {
        // Safe catch for autoplay policy restrictions or missing file errors
        console.warn(`SoundSystem: play() failed for "${soundId}":`, err.message);
      });
    } catch (err) {
      console.warn(`SoundSystem: Failed to play sound "${soundId}":`, err);
    }
  }

  /**
   * Stops a sound. If it is looping, stops the loop.
   */
  stop(soundId: SoundId): void {
    this.stopLoop(soundId);
  }

  /**
   * Plays a sound in a continuous loop.
   */
  playLoop(soundId: SoundId): void {
    if (this.activeLoops.has(soundId)) return;

    let template = this.cache.get(soundId);
    let audio: HTMLAudioElement;

    if (template) {
      audio = template.cloneNode(true) as HTMLAudioElement;
    } else {
      const path = this.soundPaths[soundId];
      audio = new Audio(path);
    }

    audio.loop = true;
    audio.volume = this.isMutedSetting ? 0 : this.volumeSetting;
    this.activeLoops.set(soundId, audio);

    audio.play().catch((err) => {
      console.warn(`SoundSystem: playLoop() failed for "${soundId}":`, err.message);
    });
  }

  /**
   * Stops a looping sound.
   */
  stopLoop(soundId: SoundId): void {
    const audio = this.activeLoops.get(soundId);
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
      this.activeLoops.delete(soundId);
    }
  }

  /**
   * Sets the volume level (0.0 to 1.0) and updates active loops.
   */
  setVolume(volume: number): void {
    this.volumeSetting = Math.max(0, Math.min(1, volume));

    // Update active loops
    for (const audio of this.activeLoops.values()) {
      audio.volume = this.isMutedSetting ? 0 : this.volumeSetting;
    }
  }

  /**
   * Mutes audio playback.
   */
  mute(): void {
    this.isMutedSetting = true;
    for (const audio of this.activeLoops.values()) {
      audio.volume = 0;
    }
  }

  /**
   * Unmutes audio playback.
   */
  unmute(): void {
    this.isMutedSetting = false;
    for (const audio of this.activeLoops.values()) {
      audio.volume = this.volumeSetting;
    }
  }
}

export const SoundSystem = new SoundSystemClass();
