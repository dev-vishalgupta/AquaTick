export default function EmptyState() {
  return (
    <div
      aria-label="No data is currently tracked"
      className="flex flex-col items-center justify-center p-8 min-h-[250px] text-center"
    >
      <div className="flex items-center justify-center w-12 h-12 bg-slate-100 dark:bg-slate-800 rounded-full text-slate-400">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={1.5}
          stroke="currentColor"
          className="w-6 h-6"
          role="img"
          aria-hidden="true"
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 3v16.5c0 .621.504 1.125 1.125 1.125h16.5c.621 0 1.125-.504 1.125-1.125V3m-18.75 0h18.75M3.75 3l7.5 7.5M20.25 3l-7.5 7.5M12 12v9" />
        </svg>
      </div>
      <h3 className="mt-4 text-sm font-semibold text-slate-800 dark:text-slate-200">No Data Available</h3>
      <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
        No active reminders or tracking logs exist for the selected range.
      </p>
    </div>
  );
}
