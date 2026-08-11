import { Outlet, useLocation } from "react-router-dom";
import NavigationBar from "./NavigationBar";
import SystemOrchestrator from "./SystemOrchestrator";

export default function AppShell() {
  const location = useLocation();

  return (
    <div className="flex w-full h-full overflow-hidden bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 transition-colors duration-200">
      {/* Background Orchestrator mounted exactly once inside the app layout root */}
      <SystemOrchestrator />
      
      {/* Persistent Left Sidebar Navigation */}
      <NavigationBar />
      
      {/* Route Content Area with vertical separation border and key-triggered page transitions */}
      <main className="flex-1 h-full overflow-hidden relative border-l border-slate-250/80 dark:border-slate-850/50">
        <div
          key={location.pathname}
          className="w-full h-full animate-fade-in"
        >
          <Outlet />
        </div>
      </main>
    </div>
  );
}
