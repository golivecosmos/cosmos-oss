import React, { useEffect, useCallback } from "react";
import { useAppLayout } from "../../contexts/AppLayoutContext";
import { useBriefing } from "../../hooks/useBriefing";
import { DashboardEmpty } from "./DashboardEmpty";
import { DashboardIndexing } from "./DashboardIndexing";
import { DashboardBriefing } from "./DashboardBriefing";

type DashboardState = "empty" | "indexing" | "briefing";

export const Dashboard: React.FC = () => {
  const {
    indexedCount,
    clusters,
    filePositions,
    hasActiveJobs,
    handleBulkIndex,
  } = useAppLayout();

  const {
    insights,
    notices,
    isLoading: isBriefingLoading,
    enrichingClusterName,
    generateBriefing,
  } = useBriefing();

  // Determine dashboard state
  const state: DashboardState =
    indexedCount === 0 && !hasActiveJobs
      ? "empty"
      : hasActiveJobs && clusters.length === 0
      ? "indexing"
      : "briefing";

  // Auto-generate briefing when clusters are available and we haven't generated yet
  useEffect(() => {
    if (clusters.length > 0 && insights.length === 0 && !isBriefingLoading) {
      generateBriefing();
    }
  }, [clusters.length, insights.length, isBriefingLoading, generateBriefing]);

  const handleSelectFolder = useCallback(
    (path: string) => {
      handleBulkIndex({ path, is_dir: true, name: path.split("/").pop() || path } as any);
    },
    [handleBulkIndex]
  );

  return (
    <div className="flex flex-col h-full">
      {state === "empty" && (
        <DashboardEmpty onSelectFolder={handleSelectFolder} />
      )}
      {state === "indexing" && <DashboardIndexing />}
      {state === "briefing" && (
        <DashboardBriefing
          clusters={clusters}
          positions={filePositions}
          insights={insights}
          notices={notices}
          enrichingClusterName={enrichingClusterName}
          indexedCount={indexedCount}
        />
      )}
    </div>
  );
};
