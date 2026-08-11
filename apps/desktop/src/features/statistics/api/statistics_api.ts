import { invoke } from "@tauri-apps/api/core";
import type { DashboardRange, DashboardResponseDTO } from "../types";

/**
 * Fetches the statistics dashboard data from SQLite via Tauri IPC.
 *
 * This matches the DashboardRange and DashboardResponseDTO contracts.
 */
export async function getDashboard(range: DashboardRange): Promise<DashboardResponseDTO> {
  return await invoke<DashboardResponseDTO>("get_dashboard", { range });
}
