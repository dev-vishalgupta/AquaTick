import React from "react";

export interface NumberInputControlProps {
  id: string;
  label: string;
  value: number;
  min?: number;
  max?: number;
  onChange: (value: number) => void;
}

export default function NumberInputControl({
  id,
  label,
  value,
  min,
  max,
  onChange,
}: NumberInputControlProps) {
  const handleDecrement = () => {
    if (min !== undefined && value <= min) return;
    onChange(value - 1);
  };

  const handleIncrement = () => {
    if (max !== undefined && value >= max) return;
    onChange(value + 1);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let val = Number(e.target.value);
    if (isNaN(val)) return;
    if (min !== undefined && val < min) val = min;
    if (max !== undefined && val > max) val = max;
    onChange(val);
  };

  return (
    <div className="flex items-center justify-between p-1">
      <label
        htmlFor={id}
        className="text-sm font-semibold text-slate-800 dark:text-slate-200"
      >
        {label}
      </label>
      <div className="flex items-center border border-slate-200 dark:border-slate-700 rounded-lg overflow-hidden bg-white dark:bg-slate-800">
        <button
          type="button"
          onClick={handleDecrement}
          disabled={min !== undefined && value <= min}
          className="px-3 py-1.5 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-transparent transition-colors font-semibold focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1 dark:focus-visible:ring-offset-slate-800"
        >
          -
        </button>
        <input
          type="number"
          id={id}
          value={value}
          onChange={handleInputChange}
          className="w-16 text-center border-none focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1 dark:focus-visible:ring-offset-slate-800 bg-transparent text-sm font-medium text-slate-800 dark:text-slate-200 font-mono [-moz-appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
        />
        <button
          type="button"
          onClick={handleIncrement}
          disabled={max !== undefined && value >= max}
          className="px-3 py-1.5 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-transparent transition-colors font-semibold focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1 dark:focus-visible:ring-offset-slate-800"
        >
          +
        </button>
      </div>
    </div>
  );
}
