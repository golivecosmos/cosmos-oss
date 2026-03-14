import React from "react";
import PreviewArea from "../PreviewArea";
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
  } = useAppLayout();

  // AI Library doesn't have a selected file
  const selectedFile = null;

  return (
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
  );
};
