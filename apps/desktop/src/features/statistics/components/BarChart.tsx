import { useState } from "react";
import type { ChartDataPointDTO } from "../types";

interface BarChartProps {
  data: ChartDataPointDTO[];
}

export default function BarChart({ data }: BarChartProps) {
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl">
        <p className="text-sm text-slate-500 dark:text-slate-405">No chart data available</p>
      </div>
    );
  }

  const height = 220;
  const paddingBottom = 30;
  const paddingTop = 20;
  const paddingLeft = 40;
  const paddingRight = 20;
  const chartHeight = height - paddingTop - paddingBottom;

  const svgWidth = 800;
  const chartWidth = svgWidth - paddingLeft - paddingRight;

  const barCount = data.length;
  const totalBarWidth = chartWidth / barCount;
  const barWidth = Math.max(8, totalBarWidth * 0.6);
  const barGap = totalBarWidth - barWidth;

  const yTicks = [0, 25, 50, 75, 100];

  return (
    <div className="p-6 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-bold text-slate-800 dark:text-slate-200">Hydration Completion Trend</h3>
        <span className="text-xs text-slate-400 dark:text-slate-500">Completion rate (%)</span>
      </div>

      <div className="relative">
        <svg
          viewBox={`0 0 ${svgWidth} ${height}`}
          className="w-full h-auto overflow-visible select-none"
          role="img"
          aria-label="Hydration Completion Trend bar chart"
        >
          {yTicks.map((tick) => {
            const y = paddingTop + chartHeight - (tick / 100) * chartHeight;
            return (
              <g key={tick} className="opacity-40 dark:opacity-20">
                <line
                  x1={paddingLeft}
                  y1={y}
                  x2={svgWidth - paddingRight}
                  y2={y}
                  stroke="currentColor"
                  strokeWidth={1}
                  strokeDasharray="4,4"
                  className="text-slate-300 dark:text-slate-700"
                />
                <text
                  x={paddingLeft - 10}
                  y={y + 4}
                  textAnchor="end"
                  fontSize={10}
                  fontWeight={500}
                  className="fill-slate-450 dark:fill-slate-500 font-sans"
                >
                  {tick}%
                </text>
              </g>
            );
          })}

          {data.map((d, idx) => {
            const pct = Math.min(100, Math.max(0, d.completionPercentage));
            const barHeight = (pct / 100) * chartHeight;
            const x = paddingLeft + idx * totalBarWidth + barGap / 2;
            const y = paddingTop + chartHeight - barHeight;
            const isHovered = hoveredIdx === idx;

            return (
              <g
                key={idx}
                onMouseEnter={() => setHoveredIdx(idx)}
                onMouseLeave={() => setHoveredIdx(null)}
                className="cursor-pointer"
              >
                <rect
                  x={x - barGap / 2}
                  y={paddingTop}
                  width={totalBarWidth}
                  height={chartHeight}
                  fill="transparent"
                />

                <rect
                  x={x}
                  y={y}
                  width={barWidth}
                  height={Math.max(2, barHeight)}
                  rx={Math.min(4, barWidth / 2)}
                  ry={Math.min(4, barWidth / 2)}
                  className={`transition-all duration-350 ${
                    isHovered
                      ? "fill-blue-500 dark:fill-blue-400"
                      : "fill-blue-600/80 dark:fill-blue-500/70"
                  }`}
                />

                <text
                  x={x + barWidth / 2}
                  y={height - 8}
                  textAnchor="middle"
                  fontSize={9}
                  fontWeight={500}
                  className="fill-slate-500 dark:fill-slate-400 font-sans"
                >
                  {d.label}
                </text>
              </g>
            );
          })}
        </svg>

        {hoveredIdx !== null && (
          <div
            className="absolute z-30 bg-slate-950/95 dark:bg-slate-900/95 text-white p-3 rounded-lg shadow-xl border border-slate-800 text-xs transition-all pointer-events-none"
            style={{
              left: `${Math.min(
                85,
                Math.max(
                  5,
                  ((paddingLeft + hoveredIdx * totalBarWidth + barWidth / 2) / svgWidth) * 100
                )
              )}%`,
              top: "-40px",
              transform: "translate(-50%, -100%)",
            }}
          >
            <div className="font-bold border-b border-slate-850 pb-1 mb-1 text-slate-350">
              {data[hoveredIdx].label}
            </div>
            <div className="space-y-0.5">
              <div>
                Completion:{" "}
                <span className="font-bold text-blue-400">
                  {data[hoveredIdx].completionPercentage.toFixed(0)}%
                </span>
              </div>
              <div className="text-slate-400">
                Completed: <span className="text-white">{data[hoveredIdx].completed}</span>
              </div>
              <div className="text-slate-400">
                Expected/Target: <span className="text-white">{data[hoveredIdx].target}</span>
              </div>
              <div className="text-slate-400">
                Snoozed: <span className="text-white">{data[hoveredIdx].snoozed}</span>
              </div>
              <div className="text-slate-400">
                Ignored: <span className="text-white">{data[hoveredIdx].ignored}</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
