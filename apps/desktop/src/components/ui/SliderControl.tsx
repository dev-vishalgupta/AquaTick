
export interface SliderControlProps {
  id: string;
  label: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  onChange: (value: number) => void;
  formatValue?: (value: number) => string;
}

export default function SliderControl({
  id,
  label,
  min,
  max,
  step = 1,
  value,
  onChange,
  formatValue,
}: SliderControlProps) {
  const displayValue = formatValue ? formatValue(value) : value.toString();

  return (
    <div className="flex flex-col gap-2 p-1">
      <div className="flex justify-between items-center">
        <label
          htmlFor={id}
          className="text-sm font-semibold text-slate-800 dark:text-slate-200"
        >
          {label}
        </label>
        <span className="text-sm font-mono font-medium text-blue-600 dark:text-blue-400">
          {displayValue}
        </span>
      </div>
      <input
        type="range"
        id={id}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full h-2 bg-slate-200 dark:bg-slate-700 rounded-lg appearance-none cursor-pointer accent-blue-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-900"
      />
    </div>
  );
}
