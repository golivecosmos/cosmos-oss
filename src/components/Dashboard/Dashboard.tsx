import React, { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
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

  // Called by DashboardEmpty after models are downloaded
  const handleStartIndexing = useCallback(async (path: string) => {
    try {
      await invoke("index_directory", { path, maxDepth: null });
    } catch (e) {
      console.error("Failed to start indexing:", e);
    }
  }, []);

  return (
    <div className="flex flex-col h-full">
      {state === "empty" && (
        <DashboardEmpty onStartIndexing={handleStartIndexing} />
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
