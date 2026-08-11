
import { APP_METADATA } from "../../../constants/metadata";

export default function AboutSection() {
  return (
    <div className="text-center md:text-left space-y-4">
      <div className="flex flex-col md:flex-row items-center md:items-start gap-4">
        <div className="flex-shrink-0 flex items-center justify-center w-12 h-12 rounded-2xl bg-blue-600 text-white font-bold text-xl shadow-md">
          AT
        </div>
        <div className="space-y-1">
          <h3 className="text-base font-bold text-slate-800 dark:text-slate-100">
            {APP_METADATA.name}
          </h3>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Version {APP_METADATA.version} ({APP_METADATA.edition})
          </p>
        </div>
      </div>
      <p className="text-sm text-slate-600 dark:text-slate-350 leading-relaxed max-w-md">
        {APP_METADATA.description}
      </p>
      <div className="pt-2 border-t border-slate-100 dark:border-slate-800/60 flex flex-wrap gap-x-4 gap-y-1 text-xs text-slate-400 dark:text-slate-500">
        <span>{APP_METADATA.copyright}</span>
        <span>•</span>
        <a href={APP_METADATA.links.privacy} className="hover:text-blue-500 transition-colors">Privacy Policy</a>
        <span>•</span>
        <a href={APP_METADATA.links.terms} className="hover:text-blue-500 transition-colors">Terms of Service</a>
      </div>
    </div>
  );
}
