/**
 * @file types.ts
 *
 * Strongly typed interface and sound definitions for the Sound System.
 */

export type SoundId =
  | "footsteps"
  | "pickup"
  | "drink"
  | "notification"
  | "click";

export interface SoundSystemAPI {
  /**
   * Plays a sound once.
   * If the sound is already playing, it will play a concurrent instance.
   *
   * @param soundId The identifier of the sound to play.
   */
  play(soundId: SoundId): void;

  /**
   * Stops the sound. If it is a looping sound, stops the loop.
   *
   * @param soundId The identifier of the sound to stop.
   */
  stop(soundId: SoundId): void;

  /**
   * Plays a sound in a continuous loop.
   *
   * @param soundId The identifier of the sound to loop.
   */
  playLoop(soundId: SoundId): void;

  /**
   * Stops a looping sound.
   *
   * @param soundId The identifier of the sound loop to stop.
   */
  stopLoop(soundId: SoundId): void;

  /**
   * Sets the playback volume for all future playbacks and active loops.
   *
   * @param volume A value between 0.0 (silent) and 1.0 (loudest).
   */
  setVolume(volume: number): void;

  /**
   * Mutes all audio output.
   */
  mute(): void;

  /**
   * Unmutes all audio output.
   */
  unmute(): void;
}
