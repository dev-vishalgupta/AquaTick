import React from "react";

interface SectionCardProps {
  children: React.ReactNode;
}

export default function SectionCard({ children }: SectionCardProps) {
  return (
    <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl p-5 shadow-sm space-y-4 hover:shadow-md transition-shadow">
      {children}
    </div>
  );
}
