import { create } from "zustand";

interface AppState {
  version: string;
  theme: "system";
  isInitialized: boolean;
  setInitialized: (initialized: boolean) => void;
}

/**
 * AquaTick Zustand global store configuration.
 * Exposes core application foundation state for Phase 1.
 */
export const useAppStore = create<AppState>((set) => ({
  version: "0.1.0",
  theme: "system",
  isInitialized: false,
  setInitialized: (initialized) => set({ isInitialized: initialized }),
}));
