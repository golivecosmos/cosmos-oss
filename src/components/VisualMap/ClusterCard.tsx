import React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Badge } from "../ui/badge";
import { clusterColor } from "./VisualMap";
import type { FileCluster, FilePosition2D } from "../../hooks/useClusters";

interface ClusterCardProps {
  cluster: FileCluster;
  files: FilePosition2D[];
  isSelected: boolean;
  onClick: () => void;
}

const TYPE_ICONS: Record<string, string> = {
  image: "🖼",
  video: "🎬",
  audio: "🎵",
  document: "📄",
  mixed: "📁",
};

export const ClusterCard: React.FC<ClusterCardProps> = ({
  cluster,
  files,
  isSelected,
  onClick,
}) => {
  // Pick up to 4 image files for the mosaic
  const imageFiles = files
    .filter((f) => f.source_type === "image")
    .slice(0, 4);

  const color = clusterColor(cluster.cluster_id);

  return (
    <button
      onClick={onClick}
      className={`
        group relative w-full text-left rounded-lg border p-3 transition-all
        hover:shadow-md hover:border-ring
        ${isSelected ? "ring-2 ring-ring border-ring shadow-md" : "border-border"}
      `}
    >
      {/* Color accent bar */}
      <div
        className="absolute left-0 top-3 bottom-3 w-1 rounded-full"
        style={{ backgroundColor: color }}
      />

      <div className="pl-3 space-y-2">
        {/* Thumbnail mosaic */}
        {imageFiles.length > 0 && (
          <div className="grid grid-cols-2 gap-0.5 rounded-md overflow-hidden aspect-[2/1]">
            {imageFiles.map((f) => (
              <img
                key={f.file_id}
                src={convertFileSrc(f.file_path)}
                alt=""
                className="w-full h-full object-cover"
                loading="lazy"
              />
            ))}
            {/* Fill remaining cells with color blocks */}
            {Array.from({ length: Math.max(0, 4 - imageFiles.length) }).map(
              (_, i) => (
                <div
                  key={`fill-${i}`}
                  className="w-full h-full"
                  style={{ backgroundColor: `${color}20` }}
                />
              )
            )}
          </div>
        )}

        {/* Name + count */}
        <div className="flex items-start justify-between gap-2">
          <h3 className="text-sm font-medium leading-tight truncate">
            {cluster.name}
          </h3>
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            {cluster.file_count} {cluster.file_count === 1 ? "file" : "files"}
          </span>
        </div>

        {/* Type badge + tags */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
            {TYPE_ICONS[cluster.dominant_type] || "📁"} {cluster.dominant_type}
          </Badge>
          {cluster.auto_tags.slice(0, 2).map((tag) => (
            <Badge key={tag} variant="outline" className="text-[10px] px-1.5 py-0">
              {tag}
            </Badge>
          ))}
        </div>
      </div>
    </button>
  );
};
