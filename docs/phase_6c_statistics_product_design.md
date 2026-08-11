# Phase 6C: Statistics Product Design

## 1. Product Vision Paradigm
AquaTick is explicitly designed as a **hydration reminder application**, not a volume-based water intake tracker. As such:
- The previous setting `daily_goal_ml` is deprecated and no longer used in the product. It will remain in the database for backward compatibility.
- Statistics and targets are now exclusively based on **reminder completion** rather than milliliters consumed.

## 2. Target Calculation Modes
The daily goal is expressed entirely as a count of reminders to complete. This target can be set via two distinct modes:

### Automatic Mode (Default)
The daily target scales dynamically based on how long the user is actually working at their computer.
- **Working Time Definition**: The duration the computer is ON, excluding Sleep, Hibernate, and Locked states.
- **Important Distinction**: Active keyboard/mouse input is **NOT** used to define working time. If the computer is unlocked and awake (e.g., reading a document, watching a tutorial), it counts as working time.
- **Formula**: `Automatic Target = floor(Working Time / Reminder Interval)`
- **Constraint**: If Working Time > 0, the target is at minimum 1.

### Manual Mode
The user specifies a fixed target for daily reminders, ignoring computer working time.
- **Settings Added**: `target_mode` (Automatic | Manual) and `manual_target_reminders` (Integer).

## 3. Summary Cards
The dashboard top section will display the following summary metrics for today:
1. **Completion Rate**: The percentage of completed reminders relative to the target.
2. **Completed Reminders**: The absolute count of completed hydration sessions today.
3. **Target**: The expected number of reminders for today.
4. **Snoozed**: Total count of snoozed reminders today.
5. **Ignored**: Total count of ignored (missed) reminders today.
6. **Working Time**: Time the system has been active today.
7. **Reminder Interval**: The current interval setting (e.g., "45 mins").
8. **Target Mode**: "Automatic" or "Manual".

## 4. Primary Visualizations
The main area of the dashboard features a bar chart with a Y-axis representing **Completion Percentage**.

- **Daily Graph**:
  - One reminder equals one bar.
  - Bars are ordered chronologically by reminder trigger.
  - The tooltip displays:
    - Trigger time
    - Completed
    - Snoozed
    - Ignored
    - Completion status
  - Do not use hour buckets.
- **Weekly Graph**:
  - X-Axis: Days of the week (Mon-Sun).
  - Y-Axis: Completion Percentage.
  - Scale: One day = one bar.
- **Monthly Graph**:
  - X-Axis: Deterministic groupings of days.
  - Y-Axis: Average Completion Percentage.
  - Scale:
    - Days 1–3
    - Days 4–6
    - Days 7–9
    ...
  The grouping must always be deterministic.

## 5. Consistency & Streaks
A dedicated section focuses on habit-building and consistency.

- **GitHub-Style Heatmap**:
  - Exclusively represents daily consistency and streak maintenance over a long timeframe (e.g., 6–12 months).
  - Does **not** display granular reminder statistics.
  - Color intensity correlates with daily completion percentage.
- **Streaks**:
  - **Threshold**: A day is marked "successful" and counts toward a streak if the Completion Rate is **≥ 80%**.
  - **Metrics Displayed**: Current Streak and Longest Streak.
  - **Grace Days**: Days where the user records zero working time (e.g., weekends away from the computer) do **not** break an active streak.
