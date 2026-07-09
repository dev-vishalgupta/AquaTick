/**
 * Local storage adapter boilerplate.
 * Prepared for Phase 2 state and settings local persistence syncing.
 */
export const storage = {
  getItem(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch (e) {
      console.error("Failed to read from local storage", e);
      return null;
    }
  },

  setItem(key: string, value: string): void {
    try {
      localStorage.setItem(key, value);
    } catch (e) {
      console.error("Failed to write to local storage", e);
    }
  },

  removeItem(key: string): void {
    try {
      localStorage.removeItem(key);
    } catch (e) {
      console.error("Failed to remove from local storage", e);
    }
  },

  clear(): void {
    try {
      localStorage.clear();
    } catch (e) {
      console.error("Failed to clear local storage", e);
    }
  },
};
