//! Data models (DTOs) for the Statistics Dashboard.
//!
//! These are read-only IPC transfer models only.
//! They are not persisted and are not database models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DashboardRange {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDTO {
    pub completion_rate: f64,
    pub completed_reminders: i64,
    pub target_reminders: i64,
    pub snoozed_reminders: i64,
    pub ignored_reminders: i64,
    pub working_time_ms: i64,
    pub reminder_interval: i64,
    pub target_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataPointDTO {
    pub label: String,
    pub completion_percentage: f64,
    pub completed: i64,
    pub target: i64,
    pub snoozed: i64,
    pub ignored: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakDataDTO {
    pub current_streak: i64,
    pub longest_streak: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDayDTO {
    pub date: String,
    pub completion_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardResponseDTO {
    pub summary: DashboardSummaryDTO,
    pub chart: Vec<ChartDataPointDTO>,
    pub heatmap: Vec<HeatmapDayDTO>,
    pub streak: StreakDataDTO,
}
