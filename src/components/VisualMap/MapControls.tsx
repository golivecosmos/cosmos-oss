import React from "react";
import { ZoomIn, ZoomOut, Maximize, Tag } from "lucide-react";

interface MapControlsProps {
  onZoomIn: () => void;
  onZoomOut: () => void;
  onResetView: () => void;
  showLabels: boolean;
  onToggleLabels: () => void;
  isLoading: boolean;
  onRecompute: () => void;
  fileCount: number;
  clusterCount: number;
}

export const MapControls: React.FC<MapControlsProps> = ({
  onZoomIn,
  onZoomOut,
  onResetView,
  showLabels,
  onToggleLabels,
  isLoading,
  onRecompute,
  fileCount,
  clusterCount,
}) => {
  return (
    <div className="absolute bottom-3 left-3 flex items-center gap-1.5 z-10">
      {/* Zoom controls */}
      <div className="flex items-center bg-background/90 backdrop-blur-sm border rounded-lg shadow-sm">
        <button
          onClick={onZoomIn}
          className="p-1.5 hover:bg-accent rounded-l-lg transition-colors"
          title="Zoom in (+)"
        >
          <ZoomIn className="w-4 h-4" />
        </button>
        <button
          onClick={onZoomOut}
          className="p-1.5 hover:bg-accent transition-colors"
          title="Zoom out (-)"
        >
          <ZoomOut className="w-4 h-4" />
        </button>
        <button
          onClick={onResetView}
          className="p-1.5 hover:bg-accent rounded-r-lg transition-colors"
          title="Reset view (0)"
        >
          <Maximize className="w-4 h-4" />
        </button>
      </div>

      {/* Toggle labels */}
      <button
        onClick={onToggleLabels}
        className={`p-1.5 rounded-lg border shadow-sm transition-colors backdrop-blur-sm ${
          showLabels
            ? "bg-primary/10 border-primary/30 text-primary"
            : "bg-background/90 hover:bg-accent"
        }`}
        title="Toggle cluster labels"
      >
        <Tag className="w-4 h-4" />
      </button>

      {/* Stats + recompute */}
      <div className="flex items-center gap-2 bg-background/90 backdrop-blur-sm border rounded-lg shadow-sm px-2.5 py-1">
        <span className="text-xs text-muted-foreground">
          {clusterCount} clusters · {fileCount} files
        </span>
        <button
          onClick={onRecompute}
          disabled={isLoading}
          className="text-xs text-primary hover:underline disabled:opacity-50"
        >
          {isLoading ? "Computing…" : "Recompute"}
        </button>
      </div>
    </div>
  );
};
