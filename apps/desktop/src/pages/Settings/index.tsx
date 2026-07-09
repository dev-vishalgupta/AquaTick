import { Link } from "react-router-dom";

export default function Settings() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-6 text-center">
      <h1 className="text-3xl font-bold tracking-tight text-slate-800 dark:text-slate-100">
        Settings
      </h1>
      <p className="mt-2 text-slate-500 dark:text-slate-400">
        Configuration settings dashboard (Placeholder)
      </p>
      <div className="mt-6">
        <Link
          to="/"
          className="px-4 py-2 text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white rounded-md transition-colors"
        >
          Back to Dashboard
        </Link>
      </div>
    </div>
  );
}
