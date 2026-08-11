import { useState } from "react";
import type { HeatmapDayDTO } from "../types";

interface HeatmapProps {
  heatmap: HeatmapDayDTO[];
}

export default function Heatmap({ heatmap }: HeatmapProps) {
  const [hoveredDay, setHoveredDay] = useState<HeatmapDayDTO | null>(null);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  if (heatmap.length === 0) {
    return (
      <div className="flex items-center justify-center h-48 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl">
        <p className="text-sm text-slate-500 dark:text-slate-400">No heatmap data available</p>
      </div>
    );
  }

  const cols: HeatmapDayDTO[][] = [];
  let currentCol: HeatmapDayDTO[] = [];

  heatmap.forEach((day, idx) => {
    currentCol.push(day);
    if (currentCol.length === 7 || idx === heatmap.length - 1) {
      cols.push(currentCol);
      currentCol = [];
    }
  });

  const getCellClass = (pct: number) => {
    if (pct === 0) return "bg-slate-100 dark:bg-slate-800/80 hover:ring-2 hover:ring-slate-300 dark:hover:ring-slate-600";
    if (pct < 30) return "bg-blue-100 dark:bg-blue-950/40 text-blue-805 hover:ring-2 hover:ring-blue-300";
    if (pct < 60) return "bg-blue-300 dark:bg-blue-850/60 text-blue-900 hover:ring-2 hover:ring-blue-400";
    if (pct < 80) return "bg-blue-500 dark:bg-blue-700/80 text-blue-100 hover:ring-2 hover:ring-blue-300";
    return "bg-emerald-500 dark:bg-emerald-600/90 text-emerald-50 hover:ring-2 hover:ring-emerald-405";
  };

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="p-6 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-6">
        <div>
          <h3 className="text-sm font-bold text-slate-800 dark:text-slate-200">180-Day Habit Grid</h3>
          <p className="text-xs text-slate-500 dark:text-slate-400">Green days represent successful targets reached (≥ 80% completion).</p>
        </div>
        
        <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400">
          <span>Less</span>
          <div className="w-3 h-3 rounded bg-slate-100 dark:bg-slate-850" />
          <div className="w-3 h-3 rounded bg-blue-100 dark:bg-blue-950/40" />
          <div className="w-3 h-3 rounded bg-blue-300 dark:bg-blue-800" />
          <div className="w-3 h-3 rounded bg-blue-500 dark:bg-blue-700" />
          <div className="w-3 h-3 rounded bg-emerald-500 dark:bg-emerald-600" />
          <span>More</span>
        </div>
      </div>

      <div
        className="relative overflow-x-auto pb-2"
        aria-label="180-Day Habit Grid"
      >
        <div className="flex gap-1.5 min-w-max select-none">
          <div className="flex flex-col justify-between text-[9px] font-semibold text-slate-400 dark:text-slate-550 pr-2 pt-1 h-[98px]">
            <span>Mon</span>
            <span>Wed</span>
            <span>Fri</span>
            <span>Sun</span>
          </div>

          {cols.map((col, colIdx) => (
            <div key={colIdx} className="flex flex-col gap-1.5">
              {col.map((day, rowIdx) => {
                const globalIdx = colIdx * 7 + rowIdx;
                const dateText = formatDate(day.date);
                const titleText = `${dateText}: ${day.completionPercentage.toFixed(0)}% completion`;
                return (
                  <div
                    key={rowIdx}
                    onMouseEnter={() => {
                      setHoveredDay(day);
                      setHoveredIdx(globalIdx);
                    }}
                    onMouseLeave={() => {
                      setHoveredDay(null);
                      setHoveredIdx(null);
                    }}
                    title={titleText}
                    aria-label={titleText}
                    className={`w-3.5 h-3.5 rounded transition-colors cursor-pointer ${getCellClass(
                      day.completionPercentage
                    )}`}
                  />
                );
              })}
            </div>
          ))}
        </div>

        {hoveredDay !== null && hoveredIdx !== null && (
          <div
            className="absolute z-30 bg-slate-950/95 dark:bg-slate-900/95 text-white py-1.5 px-3 rounded shadow-lg text-[10px] border border-slate-800 pointer-events-none"
            style={{
              left: `${Math.min(90, Math.max(5, (Math.floor(hoveredIdx / 7) / cols.length) * 100))}%`,
              top: `${Math.min(50, (hoveredIdx % 7) * 16)}px`,
              transform: "translate(-50%, -125%)",
            }}
          >
            <div className="font-bold">{formatDate(hoveredDay.date)}</div>
            <div className="mt-0.5">
              Completion:{" "}
              <span className={hoveredDay.completionPercentage >= 80 ? "text-emerald-400 font-bold" : "text-blue-400 font-bold"}>
                {hoveredDay.completionPercentage.toFixed(0)}%
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
