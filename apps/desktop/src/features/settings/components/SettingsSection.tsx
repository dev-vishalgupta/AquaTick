import React from "react";

interface SettingsSectionProps {
  title: string;
  description?: string;
  children: React.ReactNode;
}

export default function SettingsSection({
  title,
  description,
  children,
}: SettingsSectionProps) {
  return (
    <div className="space-y-3">
      <div>
        <h2 className="text-base font-bold text-slate-800 dark:text-slate-100 tracking-tight">
          {title}
        </h2>
        {description && (
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            {description}
          </p>
        )}
      </div>
      {children}
    </div>
  );
}
