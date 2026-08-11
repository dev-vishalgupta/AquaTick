import SettingsPage from "../../features/settings/components/SettingsPage";

export default function Settings() {
  return (
    <div className="flex flex-col h-full bg-slate-50 dark:bg-slate-950">
      {/* Main Settings Content */}
      <div className="flex-1 overflow-hidden">
        <SettingsPage />
      </div>
    </div>
  );
}
