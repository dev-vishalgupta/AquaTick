
export interface ToggleControlProps {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export default function ToggleControl({
  id,
  label,
  description,
  checked,
  onChange,
  disabled = false,
}: ToggleControlProps) {
  return (
    <div className="flex items-center justify-between p-1">
      <div className="flex flex-col mr-4">
        <label
          htmlFor={id}
          className="text-sm font-semibold text-slate-800 dark:text-slate-200 cursor-pointer"
        >
          {label}
        </label>
        {description && (
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {description}
          </span>
        )}
      </div>
      <button
        id={id}
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`
          relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent 
          transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-900
          ${checked ? "bg-blue-600" : "bg-slate-200 dark:bg-slate-700"}
          ${disabled ? "opacity-50 cursor-not-allowed" : ""}
        `}
      >
        <span
          className={`
            pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 
            transition duration-200 ease-in-out
            ${checked ? "translate-x-5" : "translate-x-0"}
          `}
        />
      </button>
    </div>
  );
}
