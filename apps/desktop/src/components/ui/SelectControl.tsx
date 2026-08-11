
export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectControlProps {
  id: string;
  label: string;
  options: SelectOption[];
  value: string;
  onChange: (value: string) => void;
}

export default function SelectControl({
  id,
  label,
  options,
  value,
  onChange,
}: SelectControlProps) {
  return (
    <div className="flex items-center justify-between p-1">
      <label
        htmlFor={id}
        className="text-sm font-semibold text-slate-800 dark:text-slate-200"
      >
        {label}
      </label>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-40 px-3 py-1.5 text-sm font-medium text-slate-800 dark:text-slate-200 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-900 cursor-pointer transition-shadow"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
