import type { DashboardRange } from "../types";

interface DashboardHeaderProps {
  range: DashboardRange;
  onRangeChange: (range: DashboardRange) => void;
}

export default function DashboardHeader({ range, onRangeChange }: DashboardHeaderProps) {
  const options: { value: DashboardRange; label: string }[] = [
    { value: "daily", label: "Daily" },
    { value: "weekly", label: "Weekly" },
    { value: "monthly", label: "Monthly" },
  ];

  return (
    <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 py-4 px-6 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800">
      <div>
        <h2 className="text-xl font-bold text-slate-800 dark:text-slate-200">Consistency Insights</h2>
        <p className="text-xs text-slate-500 dark:text-slate-400">Track and optimize your hydration habits.</p>
      </div>

      <div
        role="group"
        aria-label="Select dashboard range"
        className="flex bg-slate-100 dark:bg-slate-800 p-1 rounded-lg border border-slate-200 dark:border-slate-700"
      >
        {options.map((opt) => {
          const isPressed = range === opt.value;
          return (
            <button
              key={opt.value}
              onClick={() => onRangeChange(opt.value)}
              aria-pressed={isPressed}
              className={`px-4 py-1.5 text-xs font-semibold rounded-md transition-all focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-800 outline-none ${
                isPressed
                  ? "bg-white dark:bg-slate-950 text-blue-600 dark:text-blue-400 shadow-sm"
                  : "text-slate-650 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
              }`}
            >
              {opt.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
