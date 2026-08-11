import SummaryCards from "./SummaryCards";
import BarChart from "./BarChart";
import Heatmap from "./Heatmap";
import StreakCards from "./StreakCards";
import type { DashboardResponseDTO } from "../types";

interface DashboardContentProps {
  data: DashboardResponseDTO;
}

export default function DashboardContent({ data }: DashboardContentProps) {
  return (
    <div className="flex flex-col gap-6 max-w-7xl mx-auto pb-12">
      <SummaryCards summary={data.summary} />
      
      <div className="px-6">
        <BarChart data={data.chart} />
      </div>

      <div className="px-6">
        <Heatmap heatmap={data.heatmap} />
      </div>

      <StreakCards streak={data.streak} />
    </div>
  );
}
