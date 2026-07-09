import { Link } from "react-router-dom";

export default function Dashboard() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-6 text-center">
      <h1 className="text-3xl font-bold tracking-tight text-blue-600 dark:text-blue-400">
        AquaTick Dashboard
      </h1>
      <p className="mt-2 text-slate-500 dark:text-slate-400">Phase 1 Project Foundation Setup</p>
      <div className="flex gap-4 mt-6">
        <Link
          to="/settings"
          className="px-4 py-2 text-sm font-medium bg-slate-200 hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 rounded-md transition-colors"
        >
          Go to Settings
        </Link>
        <Link
          to="/about"
          className="px-4 py-2 text-sm font-medium bg-slate-200 hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 rounded-md transition-colors"
        >
          Go to About
        </Link>
      </div>
    </div>
  );
}
