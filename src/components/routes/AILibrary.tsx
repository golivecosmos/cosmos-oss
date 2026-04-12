import React, { useState, useCallback } from "react";
import { LayoutGrid, Map } from "lucide-react";
import PreviewArea from "../PreviewArea";
import { VisualMap } from "../VisualMap/VisualMap";
import { MapControls } from "../VisualMap/MapControls";
import { useAppLayout } from "../../contexts/AppLayoutContext";

export const AILibrary: React.FC = () => {
  const {
    selectedCollection,
    indexingPaths,
    transcribingPaths,
    searchState,
    indexedCount,
    showReferenceImage,
    referenceImage,
    handleReferenceImageClose,
    refreshCurrentSearch,
    handleAddToIndex,
    handleBulkIndex,
    handleTranscribeFile,
    isIndexingAllowed,
    handleSearch,
    handleFileUpload,
    setReferenceImage,
    setShowReferenceImage,
    clearSearch,
    hasActiveJobs,
    hasFailedJobs,
    setShowIndexingInfo,
    // Cluster state
    clusters,
    filePositions,
    selectedClusterId,
    setSelectedClusterId,
    isClustering,
    recomputeClusters,
    viewMode,
    setViewMode,
  } = useAppLayout();

  const [showLabels, setShowLabels] = useState(true);

  const handleSelectFile = useCallback((_fileId: string, filePath: string) => {
    // TODO: open file in preview panel
    console.log("Selected file:", filePath);
  }, []);

  const handleZoomIn = useCallback(() => {
    // Dispatch a synthetic keyboard event to the map
    const el = document.querySelector("[data-visual-map]");
    if (el) el.dispatchEvent(new KeyboardEvent("keydown", { key: "+" }));
  }, []);

  const handleZoomOut = useCallback(() => {
    const el = document.querySelector("[data-visual-map]");
    if (el) el.dispatchEvent(new KeyboardEvent("keydown", { key: "-" }));
  }, []);

  const handleResetView = useCallback(() => {
    const el = document.querySelector("[data-visual-map]");
    if (el) el.dispatchEvent(new KeyboardEvent("keydown", { key: "0" }));
  }, []);

  // AI Library doesn't have a selected file
  const selectedFile = null;

  return (
    <div className="flex flex-col h-full">
      {/* View toggle header */}
      <div className="flex items-center justify-between px-4 py-2 border-b">
        <div className="flex items-center gap-1 bg-muted rounded-lg p-0.5">
          <button
            onClick={() => setViewMode("grid")}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-sm transition-colors ${
              viewMode === "grid"
                ? "bg-background shadow-sm font-medium"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <LayoutGrid className="w-3.5 h-3.5" />
            Grid
          </button>
          <button
            onClick={() => setViewMode("map")}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-sm transition-colors ${
              viewMode === "map"
                ? "bg-background shadow-sm font-medium"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Map className="w-3.5 h-3.5" />
            Map
          </button>
        </div>

        {viewMode === "map" && clusters.length > 0 && selectedClusterId !== null && (
          <button
            onClick={() => setSelectedClusterId(null)}
            className="text-xs text-muted-foreground hover:text-foreground"
          >
            Clear selection
          </button>
        )}
      </div>

      {/* Content area */}
      <div className="flex-1 min-h-0">
        {viewMode === "grid" ? (
          <PreviewArea
            selectedFile={selectedFile}
            selectedCollection={selectedCollection}
            indexingPaths={indexingPaths}
            transcribingPaths={transcribingPaths}
            onAddToIndex={handleAddToIndex}
            onTranscribeFile={handleTranscribeFile}
            onBulkIndex={(path) =>
              handleBulkIndex({ path, is_dir: true } as any)
            }
            isIndexingDisabled={!isIndexingAllowed()}
            showReferenceImage={showReferenceImage}
            referenceImage={referenceImage}
            searchState={searchState}
            totalCount={indexedCount}
            onReferenceImageClose={handleReferenceImageClose}
            onRefreshSearch={refreshCurrentSearch}
            handleSearch={handleSearch}
            handleFileUpload={handleFileUpload}
            setReferenceImage={setReferenceImage}
            setShowReferenceImage={setShowReferenceImage}
            clearSearch={clearSearch}
            hasActiveJobs={hasActiveJobs}
            hasFailedJobs={hasFailedJobs}
            setShowIndexingInfo={setShowIndexingInfo}
          />
        ) : (
          <div className="relative w-full h-full" data-visual-map>
            <VisualMap
              clusters={clusters}
              positions={filePositions}
              selectedClusterId={selectedClusterId}
              onSelectCluster={setSelectedClusterId}
              onSelectFile={handleSelectFile}
              showLabels={showLabels}
            />
            <MapControls
              onZoomIn={handleZoomIn}
              onZoomOut={handleZoomOut}
              onResetView={handleResetView}
              showLabels={showLabels}
              onToggleLabels={() => setShowLabels((s) => !s)}
              isLoading={isClustering}
              onRecompute={recomputeClusters}
              fileCount={filePositions.length}
              clusterCount={clusters.length}
            />
          </div>
        )}
      </div>
    </div>
  );
};
