import { APP_METADATA } from "../../constants/metadata";

export default function About() {
  const features = [
    {
      title: "Consistency Insights",
      description: "Track and optimize your hydration patterns with multi-range history trends.",
      icon: "📊"
    },
    {
      title: "Interactive Companions",
      description: "Select and scale desktop companion helpers with real-time viewport previews.",
      icon: "🐧"
    },
    {
      title: "Smart Alerts",
      description: "Receive timely audio-visual notifications based on computer activity metrics.",
      icon: "🔔"
    },
    {
      title: "Local Durability",
      description: "Your tracking data is secured locally using an embedded SQLite repository layer.",
      icon: "💾"
    }
  ];

  return (
    <div className="w-full h-full max-h-screen overflow-y-auto px-6 py-10 md:px-12 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 custom-scrollbar transition-colors">
      <div className="max-w-2xl mx-auto space-y-10 pb-12">
        {/* Brand Banner */}
        <div className="flex flex-col items-center text-center space-y-4">
          <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-blue-600 text-white font-black text-3xl shadow-lg border border-blue-500/20 transform hover:rotate-6 transition-transform">
            AT
          </div>
          <div>
            <h1 className="text-3xl font-extrabold tracking-tight text-slate-800 dark:text-slate-100">
              {APP_METADATA.name}
            </h1>
            <p className="text-sm font-semibold text-blue-600 dark:text-blue-400 mt-1">
              Version {APP_METADATA.version} ({APP_METADATA.edition})
            </p>
          </div>
          <p className="text-sm text-slate-600 dark:text-slate-350 leading-relaxed max-w-md">
            {APP_METADATA.description}
          </p>
        </div>

        {/* Feature Grid */}
        <div className="space-y-4">
          <h2 className="text-xs font-bold text-slate-400 dark:text-slate-550 uppercase tracking-widest text-center md:text-left">
            Core Features
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {features.map((feature, idx) => (
              <div
                key={idx}
                className="flex items-start gap-4 p-4 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800/80 rounded-2xl shadow-sm hover-scale-animation"
              >
                <div className="text-2xl select-none pt-0.5">{feature.icon}</div>
                <div className="space-y-1">
                  <h3 className="text-sm font-bold text-slate-800 dark:text-slate-200">
                    {feature.title}
                  </h3>
                  <p className="text-xs text-slate-500 dark:text-slate-400 leading-normal">
                    {feature.description}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Footer/Legals */}
        <div className="pt-8 border-t border-slate-200 dark:border-slate-800/60 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-slate-400 dark:text-slate-500">
          <span>{APP_METADATA.copyright}</span>
          <div className="flex gap-4">
            <a
              href={APP_METADATA.links.privacy}
              className="hover:text-blue-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-blue-500 rounded px-1 transition-colors"
            >
              Privacy Policy
            </a>
            <span>•</span>
            <a
              href={APP_METADATA.links.terms}
              className="hover:text-blue-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-blue-500 rounded px-1 transition-colors"
            >
              Terms of Service
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
