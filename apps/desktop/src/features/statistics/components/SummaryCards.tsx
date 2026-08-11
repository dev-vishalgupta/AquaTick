import type { DashboardSummaryDTO } from "../types";

interface SummaryCardsProps {
  summary: DashboardSummaryDTO;
}

export default function SummaryCards({ summary }: SummaryCardsProps) {
  const formatMs = (ms: number): string => {
    if (ms <= 0) return "0m";
    const totalMinutes = Math.floor(ms / 60000);
    const hrs = Math.floor(totalMinutes / 60);
    const mins = totalMinutes % 60;
    if (hrs > 0) {
      return `${hrs}h ${mins}m`;
    }
    return `${mins}m`;
  };

  const cards = [
    {
      title: "Completion Rate",
      value: `${summary.completionRate.toFixed(0)}%`,
      desc: summary.completionRate >= 80 ? "Meeting 80% habit goal" : "Below 80% habit goal",
      colorClass: summary.completionRate >= 80 ? "text-emerald-600 dark:text-emerald-400" : "text-amber-500",
      icon: (
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5" aria-hidden="true">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
        </svg>
      ),
    },
    {
      title: "Completed Reminders",
      value: `${summary.completedReminders} / ${summary.targetReminders}`,
      desc: `${summary.snoozedReminders} snoozed, ${summary.ignoredReminders} ignored`,
      colorClass: "text-blue-600 dark:text-blue-450",
      icon: (
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5" aria-hidden="true">
          <path strokeLinecap="round" strokeLinejoin="round" d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0" />
        </svg>
      ),
    },
    {
      title: "Working Time Today",
      value: formatMs(summary.workingTimeMs),
      desc: summary.targetMode === "automatic" ? "Determined by active computer time" : "Ignored in manual target mode",
      colorClass: "text-purple-650 dark:text-purple-400",
      icon: (
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5" aria-hidden="true">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25M19.5 3H4.5A1.5 1.5 0 0 0 3 4.5v.75A1.5 1.5 0 0 0 4.5 6.75h15A1.5 1.5 0 0 0 21 5.25v-.75A1.5 1.5 0 0 0 19.5 3Z" />
        </svg>
      ),
    },
    {
      title: "Reminder Target Mode",
      value: summary.targetMode.charAt(0).toUpperCase() + summary.targetMode.slice(1),
      desc: `Interval: ${summary.reminderInterval} mins`,
      colorClass: "text-slate-700 dark:text-slate-350",
      icon: (
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5" aria-hidden="true">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.43l-1.003.828c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.43l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.645-.869L9.594 3.94Z" />
          <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
        </svg>
      ),
    },
  ];

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 p-6">
      {cards.map((card, idx) => (
        <div
          key={idx}
          className="flex flex-col justify-between p-5 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm hover-scale-animation"
        >
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-slate-500 dark:text-slate-400 tracking-wide uppercase">
              {card.title}
            </span>
            <div className="text-slate-400 dark:text-slate-500">{card.icon}</div>
          </div>
          <div className="mt-4">
            <span className={`text-2xl font-bold tracking-tight ${card.colorClass}`}>
              {card.value}
            </span>
            <p className="mt-1.5 text-xs text-slate-500 dark:text-slate-400 font-medium">
              {card.desc}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}
