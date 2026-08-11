interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

export default function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center p-8 min-h-[300px] text-center"
    >
      <div className="flex items-center justify-center w-12 h-12 bg-red-100 dark:bg-red-950/30 rounded-full text-red-650 dark:text-red-400">
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
          <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
        </svg>
      </div>
      <h3 className="mt-4 text-lg font-semibold text-slate-800 dark:text-slate-200">Failed to Load Dashboard</h3>
      <p className="mt-2 text-sm text-slate-500 dark:text-slate-400 max-w-md">{message}</p>
      <button
        onClick={onRetry}
        className="mt-6 px-4 py-2 text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-900 outline-none shadow-sm"
      >
        Retry
      </button>
    </div>
  );
}
