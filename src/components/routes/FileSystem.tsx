import React, { useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import PreviewArea from "../PreviewArea";
import { useAppLayout } from "../../contexts/AppLayoutContext";

export const FileSystem: React.FC = () => {
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
  } = useAppLayout();

  return (
    <PreviewArea
      selectedFile={selectedFile}
      selectedCollection="filesystem" // Override to indicate file system mode
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
  );
};
