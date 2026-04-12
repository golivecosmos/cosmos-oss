import React from "react";
import { Clock } from "lucide-react";
import { InsightCard } from "./InsightCard";
import { NeedsAttention } from "./NeedsAttention";
import type { FileCluster, FilePosition2D } from "../../hooks/useClusters";
import type { ClusterInsight, BriefingNotice } from "../../hooks/useBriefing";

interface DashboardBriefingProps {
  clusters: FileCluster[];
  positions: FilePosition2D[];
  insights: ClusterInsight[];
  notices: BriefingNotice[];
  enrichingClusterName: string | null;
  indexedCount: number;
}

export const DashboardBriefing: React.FC<DashboardBriefingProps> = ({
  clusters,
  positions,
  insights,
  notices,
  enrichingClusterName,
  indexedCount,
}) => {
  // Build a lookup for cluster files
  const clusterFiles = React.useMemo(() => {
    const map = new Map<number, FilePosition2D[]>();
    for (const pos of positions) {
      const existing = map.get(pos.cluster_id) || [];
      existing.push(pos);
      map.set(pos.cluster_id, existing);
    }
    return map;
  }, [positions]);

  // Build a lookup for insights by cluster_id
  const insightMap = React.useMemo(() => {
    const map = new Map<number, ClusterInsight>();
    for (const insight of insights) {
      map.set(insight.cluster_id, insight);
    }
    return map;
  }, [insights]);

  // Sort clusters by file count descending
  const sortedClusters = React.useMemo(
    () => [...clusters].sort((a, b) => b.file_count - a.file_count),
    [clusters]
  );

  return (
    <div className="flex-1 min-h-0 overflow-y-auto">
      <div className="max-w-4xl mx-auto px-6 py-8 space-y-8">
        {/* Hero header */}
        <div>
          <h1 className="text-xl font-semibold mb-1">
            Here's what Cosmos found
          </h1>
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <span>{indexedCount.toLocaleString()} files</span>
            <span>·</span>
            <span>{clusters.length} topics</span>
            <span>·</span>
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              Just now
            </span>
          </div>
        </div>

        {/* Cluster insight cards */}
        {sortedClusters.length > 0 && (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
            {sortedClusters.map((cluster) => (
              <InsightCard
                key={cluster.cluster_id}
                cluster={cluster}
                insight={insightMap.get(cluster.cluster_id) || null}
                files={clusterFiles.get(cluster.cluster_id) || []}
                isEnriching={enrichingClusterName === cluster.name}
              />
            ))}
          </div>
        )}

        {/* Needs Attention */}
        <NeedsAttention notices={notices} />

        {/* What's New — placeholder for now, populated after re-indexing */}
        {/* Will be driven by comparing current vs previous cluster state */}
      </div>
    </div>
  );
};
