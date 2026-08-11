export default function LoadingState() {
  return (
    <div
      role="status"
      aria-label="Loading dashboard metrics"
      className="flex flex-col items-center justify-center p-12 min-h-[300px]"
    >
      <div className="w-10 h-10 border-4 border-blue-200 border-t-blue-600 rounded-full animate-spin"></div>
      <p className="mt-4 text-sm font-medium text-slate-500 dark:text-slate-400 animate-pulse">
        Loading dashboard metrics...
      </p>
    </div>
  );
}
