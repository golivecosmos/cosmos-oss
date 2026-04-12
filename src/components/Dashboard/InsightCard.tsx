import React from "react";
import { useNavigate } from "react-router-dom";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Badge } from "../ui/badge";
import { clusterColor } from "../VisualMap/VisualMap";
import type { FileCluster, FilePosition2D } from "../../hooks/useClusters";
import type { ClusterInsight } from "../../hooks/useBriefing";

interface InsightCardProps {
  cluster: FileCluster;
  insight: ClusterInsight | null;
  files: FilePosition2D[];
  isEnriching: boolean;
}

const TYPE_ICONS: Record<string, string> = {
  image: "📸",
  video: "🎬",
  audio: "🎵",
  document: "📄",
  mixed: "📁",
};

export const InsightCard: React.FC<InsightCardProps> = ({
  cluster,
  insight,
  files,
  isEnriching,
}) => {
  const navigate = useNavigate();
  const color = clusterColor(cluster.cluster_id);

  const imageFiles = files
    .filter((f) => f.source_type === "image")
    .slice(0, 4);

  const displayName = insight?.llm_name || cluster.name;
  const displayInsight = insight?.llm_insight || null;

  const handleClick = () => {
    navigate(`/library?cluster=${cluster.cluster_id}`);
  };

  return (
    <button
      onClick={handleClick}
      className="group relative w-full text-left rounded-xl border border-border p-4 transition-all hover:shadow-lg hover:border-ring hover:-translate-y-0.5"
    >
      {/* Color accent */}
      <div
        className="absolute left-0 top-4 bottom-4 w-1 rounded-full"
        style={{ backgroundColor: color }}
      />

      <div className="pl-3 space-y-3">
        {/* Thumbnail mosaic */}
        {imageFiles.length > 0 && (
          <div className="grid grid-cols-2 gap-0.5 rounded-lg overflow-hidden aspect-[2/1]">
            {imageFiles.map((f) => (
              <img
                key={f.file_id}
                src={convertFileSrc(f.file_path)}
                alt=""
                className="w-full h-full object-cover"
                loading="lazy"
              />
            ))}
            {Array.from({ length: Math.max(0, 4 - imageFiles.length) }).map(
              (_, i) => (
                <div
                  key={`fill-${i}`}
                  className="w-full h-full"
                  style={{ backgroundColor: `${color}15` }}
                />
              )
            )}
          </div>
        )}

        {/* Name */}
        <h3 className="text-sm font-semibold leading-tight">{displayName}</h3>

        {/* Insight or shimmer */}
        {isEnriching ? (
          <div className="space-y-1.5">
            <div className="h-3 w-4/5 rounded bg-muted animate-pulse" />
            <div className="h-3 w-3/5 rounded bg-muted animate-pulse" />
          </div>
        ) : displayInsight ? (
          <p className="text-xs text-muted-foreground leading-relaxed">
            {displayInsight}
          </p>
        ) : null}

        {/* Meta */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
              {TYPE_ICONS[cluster.dominant_type] || "📁"} {cluster.dominant_type}
            </Badge>
            <Badge variant="outline" className="text-[10px] px-1.5 py-0">
              {cluster.file_count} files
            </Badge>
          </div>
          {insight && (
            <span className="text-[10px] text-muted-foreground">
              based on {cluster.file_count} files
            </span>
          )}
        </div>
      </div>
    </button>
  );
};
