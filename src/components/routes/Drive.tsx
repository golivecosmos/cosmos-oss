import React, { useMemo } from "react";
import { useParams, useSearchParams } from "react-router-dom";
import PreviewArea from "../PreviewArea";
import { useAppLayout } from "../../contexts/AppLayoutContext";

export const Drive: React.FC = () => {
  const { drive_id } = useParams<{ drive_id: string }>();
  const [searchParams] = useSearchParams();
  const path = searchParams.get("path") || "";
  
  // Derive selectedFile from URL path - memoized to prevent unnecessary re-renders
  const selectedFile = useMemo(() => {
    if (!path) return null;
    
    return {
      name: path.split("/").pop() || "",
      path: path,
      is_dir: true, // We'll determine this properly later
      size: 0,
      modified: new Date().toISOString(),
      file_type: "directory" as const
    };
  }, [path]);

  const {
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
    isIndexingAllowed,
    handleSearch,
    handleFileUpload,
    setReferenceImage,
    setShowReferenceImage,
    clearSearch,
    hasActiveJobs,
    hasFailedJobs,
    setShowIndexingInfo,
  } = useAppLayout();

  // Drive ID will be used for drive-specific navigation
  console.log("DriveRoute - Drive ID:", drive_id, "selectedFile:", selectedFile);

  return (
    <PreviewArea
      selectedFile={selectedFile}
      selectedCollection="drive" // Override to indicate drive mode
      indexingPaths={indexingPaths}
      transcribingPaths={transcribingPaths}
      onAddToIndex={handleAddToIndex}
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
  );
};
