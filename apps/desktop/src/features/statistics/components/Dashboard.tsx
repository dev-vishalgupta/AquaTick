import { useState, useEffect } from "react";
import DashboardHeader from "./DashboardHeader";
import DashboardContent from "./DashboardContent";
import LoadingState from "./LoadingState";
import ErrorState from "./ErrorState";
import EmptyState from "./EmptyState";
import { getDashboard } from "../api/statistics_api";
import type { DashboardRange, DashboardResponseDTO } from "../types";

export default function Dashboard() {
  const [range, setRange] = useState<DashboardRange>("weekly");
  const [data, setData] = useState<DashboardResponseDTO | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const fetchDashboardData = async (currentRange: DashboardRange) => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await getDashboard(currentRange);
      setData(response);
    } catch (err: any) {
      console.error(err);
      setError(err?.message || "An unexpected error occurred while loading dashboard metrics.");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchDashboardData(range);
  }, [range]);

  return (
    <div className="flex flex-col h-full">
      <DashboardHeader range={range} onRangeChange={setRange} />

      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <LoadingState />
        ) : error ? (
          <ErrorState message={error} onRetry={() => fetchDashboardData(range)} />
        ) : data ? (
          <DashboardContent data={data} />
        ) : (
          <EmptyState />
        )}
      </div>
    </div>
  );
}
