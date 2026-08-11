# Phase 6C: Statistics Architecture

## 1. Core Principles
The architecture strictly follows the existing application layering:
**React → IPC → Services → Repositories → SQLite**

No business logic is permitted in the frontend. All aggregation, calculation, and database operations occur in the Rust backend.

## 2. Data Ownership
- **StatisticsService** owns:
  - Completion calculations
  - Target calculations
  - Streak calculations
  - Heatmap calculations
  - Dashboard DTO generation
- **SettingsService** owns:
  - Reminder interval
  - Target mode
  - Manual target
- **Repositories** own:
  - Database CRUD only

  Repositories MUST NOT perform:
  - aggregations
  - statistics calculations
  - streak calculations
  - target calculations
  - business logic

  These responsibilities belong exclusively to StatisticsService.
- **React** owns:
  - Rendering only
  - Local presentation state

## 3. File Structure

### Backend (Rust)
- **Modified**: `src-tauri/src/constants/mod.rs` (Add `TARGET_MODE`, `MANUAL_TARGET_REMINDERS`, leave `DAILY_GOAL_ML` untouched)
- **Modified**: `src-tauri/src/models/setting.rs` (Update `AppSettings` struct)
- **Modified**: `src-tauri/src/database/migrations.rs` (Add Migration V4 for new settings. Leave `daily_goal_ml` untouched)
- **Modified**: `src-tauri/src/services/settings_service.rs` (Validation for new settings)
- **Modified**: `src-tauri/src/services/statistics_service.rs` (New aggregation logic for graphs and streaks)
- **Modified**: `src-tauri/src/commands/statistics.rs` (New IPC commands for dashboard data)
- **New**: `src-tauri/src/models/dashboard.rs` (DTOs for chart and summary data)

### Frontend (React/TypeScript)
- **Modified**: `apps/desktop/src/features/settings/types.ts`
- **Modified**: `apps/desktop/src/features/settings/api/settings_api.ts`
- **Modified**: `apps/desktop/src/features/settings/components/SettingsPage.tsx`
- **New**: `apps/desktop/src/features/statistics/api/statistics_api.ts`
- **New**: `apps/desktop/src/features/statistics/components/Dashboard.tsx`
- **New**: `apps/desktop/src/features/statistics/components/SummaryCards.tsx`
- **New**: `apps/desktop/src/features/statistics/components/BarChartWidget.tsx`
- **New**: `apps/desktop/src/features/statistics/components/ConsistencyHeatmap.tsx`
- **New**: `apps/desktop/src/features/statistics/types.ts`

## 4. Component Hierarchy
```text
Dashboard
├── SummaryCards
│   ├── MetricCard (Completion Rate)
│   ├── MetricCard (Completed / Target)
│   ├── MetricCard (Snoozed / Ignored)
│   └── MetricCard (Working Time / Interval / Mode)
├── BarChartWidget
│   └── Recharts BarChart (Toggles: Daily | Weekly | Monthly)
└── ConsistencyPanel
    ├── ConsistencyHeatmap (GitHub style)
    └── StreakSummary (Current & Longest)
```

## 5. DTOs and IPC Contracts

### Data Transfer Objects (Rust → TS)
```typescript
interface DashboardSummaryDTO {
  completionRate: number;
  completedReminders: number;
  targetReminders: number;
  snoozedReminders: number;
  ignoredReminders: number;
  workingTimeMs: number;
  reminderInterval: number;
  targetMode: "automatic" | "manual";
}

interface ChartDataPointDTO {
  label: string;
  completionPercentage: number;
  completed: number;
  target: number;
  snoozed: number;
  ignored: number;
}

interface DashboardResponseDTO {
  summary: DashboardSummaryDTO;
  chart: ChartDataPointDTO[];
  heatmap: HeatmapDayDTO[];
  streak: StreakDataDTO;
}

interface StreakDataDTO {
  currentStreak: number;
  longestStreak: number;
}

interface HeatmapDayDTO {
  date: string; // "YYYY-MM-DD"
  completionPercentage: number;
}
```

### IPC Commands
- `get_dashboard(range: String)` -> `Result<DashboardResponseDTO, CommandError>`
  - `range` is `"daily"`, `"weekly"`, or `"monthly"`.

## 6. Event Flow
1. User navigates to the Statistics Dashboard.
2. `Dashboard.tsx` mounts and calls the single `get_dashboard(range)` IPC endpoint via `statistics_api.ts`.
3. IPC commands route to `StatisticsService` and `SettingsService`. `StatisticsService` exposes one orchestration method `generate_dashboard(range)` which internally composes `build_summary()`, `build_chart()`, `build_heatmap()`, and `build_streak()`. The IPC layer remains thin and simply exposes the composed dashboard.
4. `StatisticsService` queries the SQLite database via `SessionRepository` and `StatisticsRepository`.
5. `StatisticsService` applies business logic (80% streak threshold, target calculations based on working time vs manual).
6. Data is mapped to DTOs and serialized back to the frontend.
7. React state is updated, rendering the charts, cards, and heatmap.

## 7. Target Calculation & Activity Monitor
- **Target Mode Enum**: Stored in SQLite as a string (`"automatic"` or `"manual"`).
- **Working Time Calculation**:
  - `ActivityMonitorService` is responsible ONLY for detecting computer state transitions (Running, Locked, Sleep, Hibernate, Resume). It does NOT own statistics calculations.
  - Working time is accumulated by the Statistics subsystem (or dedicated time-tracking logic) using those state transitions. Active keyboard/mouse input is not used as the threshold.
  - `StatisticsService` reads this value to compute `floor(active_ms_today / (interval_minutes * 60000))` when in automatic mode.

## 8. Implementation Order
1. **Migration & Settings Refactor**:
   - Create Migration V4 to insert `target_mode` and `manual_target_reminders`. Leave `daily_goal_ml` untouched as a deprecated setting for backward compatibility. No DELETE statements.
   - Update `SettingsService` and frontend Settings UI to ignore daily goal ML and add the new mode toggle.
2. **Backend Aggregation Services**:
   - Implement `StatisticsService` methods for summary, chart bucketing, streaks, and heatmap data.
3. **IPC Layer**:
   - Wire up the new `tauri::command` functions in `commands/statistics.rs`.
4. **Frontend API**:
   - Create `statistics_api.ts` to wrap the IPC calls.
5. **Frontend UI - Cards & Heatmap**:
   - Build `SummaryCards`, `ConsistencyHeatmap`, and `StreakSummary`.
6. **Frontend UI - Charts**:
   - Build `BarChartWidget` with Daily/Weekly/Monthly toggles using Recharts.

## 9. Verification Plan
- **Database Consistency**: Verify Migration V4 safely inserts new keys without data loss and leaves existing deprecated keys untouched.
- **Unit/Integration Testing**: Rust-side tests for `StatisticsService` ensuring the 80% streak threshold and grace days (zero working time) logic works exactly as designed.
- **Manual UI Testing**:
  - Toggle between Automatic and Manual target modes in Settings and verify the Dashboard Summary Cards update correctly.
  - Verify the Bar Chart Y-axis correctly maps to completion percentage.
  - Verify Heatmap colors reflect the underlying percentage data.
