import type { StreakDataDTO } from "../types";

interface StreakCardsProps {
  streak: StreakDataDTO;
}

export default function StreakCards({ streak }: StreakCardsProps) {
  const getEncouragingMessage = (current: number) => {
    if (current === 0) return "Start a new hydration streak today!";
    if (current < 3) return "Off to a solid start! Keep it up.";
    if (current < 7) return "Awesome consistency! You're building a real habit.";
    return "Legendary! You're a hydration master!";
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-6 pt-0">
      <div className="flex items-center gap-5 p-5 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm hover-scale-animation">
        <div className="flex items-center justify-center w-14 h-14 bg-orange-100 dark:bg-orange-950/20 rounded-xl text-orange-500">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-8 h-8" aria-hidden="true">
            <path strokeLinecap="round" strokeLinejoin="round" d="M15.362 5.214A8.252 8.252 0 0 1 12 21 8.25 8.25 0 0 1 6.038 7.047 8.287 8.287 0 0 0 9 9.601a8.983 8.983 0 0 1 3.361-6.867 8.21 8.21 0 0 0 3 2.48Z" />
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 18a3.75 3.75 0 0 0 .495-7.467 5.99 5.99 0 0 0-1.925 3.546 5.974 5.974 0 0 1-2.133-1A3.75 3.75 0 0 0 12 18Z" />
          </svg>
        </div>
        <div>
          <span className="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wide uppercase">
            Current Streak
          </span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-3xl font-bold tracking-tight text-slate-800 dark:text-slate-200">
              {streak.currentStreak}
            </span>
            <span className="text-sm font-semibold text-slate-500 dark:text-slate-400">days</span>
          </div>
          <p className="mt-1 text-xs font-medium text-slate-500 dark:text-slate-400">
            {getEncouragingMessage(streak.currentStreak)}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-5 p-5 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm hover-scale-animation">
        <div className="flex items-center justify-center w-14 h-14 bg-yellow-100 dark:bg-yellow-950/20 rounded-xl text-yellow-600 dark:text-yellow-500">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-8 h-8" aria-hidden="true">
            <path strokeLinecap="round" strokeLinejoin="round" d="M16.5 18.75h-9m9 0a3 3 0 0 1 3 3h-15a3 3 0 0 1 3-3m9 0v-3.375c0-.621-.503-1.125-1.125-1.125h-.871M7.5 18.75v-3.375c0-.621.504-1.125 1.125-1.125h.872m5.007 0V9.75m-5.007 0V9.75m5.007 0a3 3 0 0 1-5.007 0m5.007 0h-5.007m0 0h5.007m-5.007 0h5.007M9 3h6m-3 0v3.375M9 3.75h6" />
          </svg>
        </div>
        <div>
          <span className="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wide uppercase">
            Longest Streak
          </span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-3xl font-bold tracking-tight text-slate-800 dark:text-slate-200">
              {streak.longestStreak}
            </span>
            <span className="text-sm font-semibold text-slate-500 dark:text-slate-400">days</span>
          </div>
          <p className="mt-1 text-xs font-medium text-slate-500 dark:text-slate-400">
            Your personal best record! Keep pushing your boundaries.
          </p>
        </div>
      </div>
    </div>
  );
}
