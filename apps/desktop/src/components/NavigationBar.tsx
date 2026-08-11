import { Link, useLocation } from "react-router-dom";
import { NAV_ITEMS } from "./navigation";
import type { NavItem } from "./navigation";

function NavigationIcon({ iconType }: { iconType: NavItem["iconType"] }) {
  switch (iconType) {
    case "statistics":
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className="w-5 h-5"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z"
          />
        </svg>
      );
    case "settings":
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className="w-5 h-5"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.43l-1.003.828c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.43l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
          />
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
          />
        </svg>
      );
    case "about":
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className="w-5 h-5"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M11.25 11.25l.041-.02a.75.75 0 111.083.985l-.5 1.716-.04.022a.75.75 0 01-1.083-.984l.5-1.716zM12 9h.008v.008H12V9zM21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      );
    default:
      return null;
  }
}

export default function NavigationBar() {
  const location = useLocation();

  return (
    <nav
      aria-label="Main navigation"
      className="w-16 h-full flex flex-col items-center py-6 bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 select-none shrink-0 transition-colors duration-200"
    >
      {/* App Logo Indicator */}
      <div className="flex items-center justify-center w-10 h-10 mb-8 rounded-xl bg-blue-500/10 text-blue-600 dark:text-blue-400 font-bold text-lg shadow-sm border border-blue-500/20">
        AT
      </div>

      {/* Nav List */}
      <div className="flex-1 w-full px-2 space-y-4">
        {NAV_ITEMS.map((item) => {
          const isActive = location.pathname === item.to;
          const tooltipId = `tooltip-${item.label.toLowerCase()}`;
          return (
            <Link
              key={item.to}
              to={item.to}
              aria-current={isActive ? "page" : undefined}
              aria-describedby={tooltipId}
              className={`relative group flex flex-col items-center justify-center w-full py-2.5 rounded-xl transition-all duration-200 focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-900 outline-none ${
                isActive
                  ? "bg-blue-50 dark:bg-blue-950/30 text-blue-600 dark:text-blue-400"
                  : "text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-350 hover:bg-slate-50 dark:hover:bg-slate-800/40"
              }`}
            >
              {/* Active Indicator Pip */}
              {isActive && (
                <div className="absolute left-0 top-1/4 bottom-1/4 w-1 bg-blue-500 rounded-r-md" />
              )}
              
              <NavigationIcon iconType={item.iconType} />
              <span className="text-[10px] font-semibold mt-1 tracking-wide">
                {item.label}
              </span>

              {/* Tooltip on hover/focus */}
              <div
                id={tooltipId}
                role="tooltip"
                className="absolute left-16 px-2.5 py-1 text-xs font-medium text-white bg-slate-900 dark:bg-slate-800 rounded-md shadow-md opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity duration-150 pointer-events-none whitespace-nowrap z-50"
              >
                {item.label}
              </div>
            </Link>
          );
        })}
      </div>

      {/* Connection Indicator / Version status */}
      <div className="text-[9px] font-mono text-slate-400 dark:text-slate-600">
        v0.1.0
      </div>
    </nav>
  );
}
