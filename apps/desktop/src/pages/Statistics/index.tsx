import Dashboard from "@/features/statistics/components/Dashboard";

export default function StatisticsPage() {
  return (
    <div className="flex flex-col h-full bg-slate-50 dark:bg-slate-950">
      {/* Statistics Feature Component */}
      <div className="flex-1 overflow-auto">
        <Dashboard />
      </div>
    </div>
  );
}
