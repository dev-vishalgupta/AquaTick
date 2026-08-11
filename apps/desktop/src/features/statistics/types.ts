export type DashboardRange = "daily" | "weekly" | "monthly";

export interface DashboardSummaryDTO {
  completionRate: number;
  completedReminders: number;
  targetReminders: number;
  snoozedReminders: number;
  ignoredReminders: number;
  workingTimeMs: number;
  reminderInterval: number;
  targetMode: "automatic" | "manual";
}

export interface ChartDataPointDTO {
  label: string;
  completionPercentage: number;
  completed: number;
  target: number;
  snoozed: number;
  ignored: number;
}

export interface StreakDataDTO {
  currentStreak: number;
  longestStreak: number;
}

export interface HeatmapDayDTO {
  date: string; // YYYY-MM-DD
  completionPercentage: number;
}

export interface DashboardResponseDTO {
  summary: DashboardSummaryDTO;
  chart: ChartDataPointDTO[];
  heatmap: HeatmapDayDTO[];
  streak: StreakDataDTO;
}
