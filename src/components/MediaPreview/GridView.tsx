import React, { useRef, useEffect } from "react";
import { MediaFile } from "./types";
import { FileText } from "lucide-react";
import { ImagePreview } from "./ImagePreview";
import { NativeVideoThumbnail } from "./NativeVideoThumbnail";
import { PreviewContextMenu } from "./PreviewContextMenu";
import { formatFileSize } from "../../lib/utils";
import { DriveStatusIndicator } from "../DriveStatusIndicator";
import { OfflineDriveCard } from "../OfflineDriveCard";
import { useVirtualizer } from "@tanstack/react-virtual";

interface GridViewProps {
  files: MediaFile[];
  onFileSelect: (file: MediaFile) => void;
  isLoading?: boolean;
  indexingPaths?: Set<string>;
  onToggleWatch?: (path: string) => void;
  onAddToIndex?: (path: string) => void;
  onTranscribeFile?: (path: string) => void;
  isIndexingDisabled?: boolean;
  onLoadMore?: () => void;
  hasMoreFiles?: boolean;
  isLoadingMore?: boolean;
}

const Skeleton = () => (
  <div className="aspect-square bg-gradient-to-br from-gray-50 to-gray-100 rounded-xl overflow-hidden animate-pulse">
    <div className="w-full h-full bg-gradient-to-r from-gray-100 via-gray-200 to-gray-100 animate-shimmer" />
  </div>
);

// Add video frame timestamp info to tooltip
const getTooltipContent = (file: MediaFile) => {
  const tooltipItems: string[] = [];

  if (file.metadata.mimeType) {
    tooltipItems.push(`Type: ${file.metadata.mimeType}`);
  }

  if (file.metadata.size) {
    tooltipItems.push(`Size: ${formatFileSize(file.metadata.size)}`);
  }

  if (file.metadata.modified) {
    tooltipItems.push(
      `Modified: ${new Date(file.metadata.modified).toLocaleString()}`
    );
  }

  // Add drive information
  if (file.metadata.driveUuid) {
    const driveName = file.metadata.driveCustomName || file.metadata.driveName;
    tooltipItems.push(`Drive: ${driveName} (${file.metadata.driveStatus})`);
    if (file.metadata.drivePhysicalLocation) {
      tooltipItems.push(`Location: ${file.metadata.drivePhysicalLocation}`);
    }
  }

  // Add timestamp info for video frames
  if (file.metadata.isVideoFrame && file.metadata.timestampFormatted) {
    tooltipItems.push(`Timestamp: ${file.metadata.timestampFormatted}`);
  }

  if (file.metadata.score && file.metadata.score > 0) {
    tooltipItems.push(
      `Match score: ${(file.metadata.score * 100).toFixed(1)}%`
    );
  }

  return tooltipItems.join("\n");
};

export const GridView = React.memo(
  ({
    files,
    onFileSelect,
    isLoading,
    indexingPaths,
    onToggleWatch,
    onAddToIndex,
    onTranscribeFile,
    isIndexingDisabled,
    onLoadMore,
    hasMoreFiles,
    isLoadingMore,
  }: GridViewProps) => {
    const containerRef = useRef<HTMLDivElement>(null);

    // Sort files to preserve original order
    const sortedFiles = [...files].sort((a, b) => {
      // Preserve original order using originalIndex
      return (a.originalIndex ?? 0) - (b.originalIndex ?? 0);
    });

    // Virtual scrolling setup for large file lists
    // Calculate items per row based on grid columns (responsive)
    const getItemsPerRow = () => {
      if (!containerRef.current) return 8;
      const width = containerRef.current.offsetWidth;
      if (width >= 1536) return 10; // 2xl
      if (width >= 1280) return 8;  // xl
      if (width >= 1024) return 6;  // lg
      if (width >= 768) return 5;   // md
      if (width >= 640) return 4;   // sm
      return 3; // default
    };

    const itemsPerRow = getItemsPerRow();
    const rowCount = Math.ceil(sortedFiles.length / itemsPerRow);

    // Set up virtualizer for rows (not individual items)
    const rowVirtualizer = useVirtualizer({
      count: rowCount,
      getScrollElement: () => containerRef.current,
      estimateSize: () => 160, // Estimated row height (120px item + 40px padding/margin)
      overscan: 3, // Render 3 extra rows above and below viewport
    });

    // Recalculate on window resize
    useEffect(() => {
      const handleResize = () => {
        rowVirtualizer.measure();
      };
      window.addEventListener('resize', handleResize);
      return () => window.removeEventListener('resize', handleResize);
    }, [rowVirtualizer]);

    // Show skeletons while loading
    if (isLoading && files.length === 0) {
      return (
        <div className="p-6">
          <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10 gap-8">
            {Array.from({ length: 12 }).map((_, i) => (
              <div key={i} className="flex flex-col items-center" style={{ width: '120px' }}>
                <div className="w-24 h-24 bg-gray-200 dark:bg-gray-700 rounded-lg animate-pulse mb-2" />
                <div className="w-full h-3 bg-gray-200 dark:bg-gray-700 rounded animate-pulse mb-1" />
                <div className="w-2/3 h-2 bg-gray-200 dark:bg-gray-700 rounded animate-pulse" />
              </div>
            ))}
          </div>
        </div>
      );
    }

    return (
      <div ref={containerRef} className="h-full w-full overflow-y-auto">
        <div className="p-6">
          {/* Virtual scrolling container */}
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: '100%',
              position: 'relative',
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const startIndex = virtualRow.index * itemsPerRow;
              const endIndex = Math.min(startIndex + itemsPerRow, sortedFiles.length);
              const rowFiles = sortedFiles.slice(startIndex, endIndex);

              return (
                <div
                  key={virtualRow.key}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  {/* Grid section - Finder style with adjusted columns for larger thumbnails */}
                  <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10 gap-8">
                    {rowFiles.map((file, index) => {
                      const globalIndex = startIndex + index;
                      // Check if the file is a grouped video
                      const isGroupedVideo = file.type === 'video' && file.metadata.isGroupedVideo;
                      const frameCount = file.metadata.frameCount || 0;

                      // Check if this is from an offline drive
                      const isOfflineDrive = file.metadata.driveStatus === 'disconnected';

                      // Show offline drive card for disconnected drives
                      if (isOfflineDrive) {
                        return (
                          <div key={`${file.path}-${globalIndex}`} className="col-span-1">
                            <OfflineDriveCard
                              file={{
                                id: file.path,
                                file_path: file.path.replace('asset://localhost/', ''),
                                metadata: file.metadata.parentPath ? JSON.stringify({ fs_size: file.metadata.size }) : '{}',
                                score: file.metadata.score || 0,
                                drive_uuid: file.metadata.driveUuid,
                                drive_name: file.metadata.driveName,
                                drive_custom_name: file.metadata.driveCustomName,
                                drive_physical_location: file.metadata.drivePhysicalLocation,
                                drive_status: file.metadata.driveStatus,
                                mime_type: file.metadata.mimeType,
                                created_at: file.metadata.created || '',
                                timestamp: file.metadata.timestamp,
                                video_duration: file.metadata.videoDuration
                              }}
                              onClick={() => {
                                // Show connect drive message
                                alert(`Please connect the drive "${file.metadata.driveCustomName || file.metadata.driveName}" to access this file.`);
                              }}
                            />
                          </div>
                        );
                      }

                      return (
                        <PreviewContextMenu
                          key={`${file.path}-${globalIndex}`}
                          file={file}
                          onAddToIndex={onAddToIndex}
                          onTranscribeFile={onTranscribeFile}
                          indexingPaths={indexingPaths}
                          isFromSearch={file.metadata.score !== undefined && file.metadata.score > 0}
                        >
                          <div
                            className="group flex flex-col items-center cursor-pointer transition-all duration-150 opacity-90 hover:opacity-100"
                            onClick={() => onFileSelect(file)}
                            title={getTooltipContent(file)}
                            style={{ width: '120px' }}
                          >
                            {/* Icon/Thumbnail area - Finder style with larger thumbnails */}
                            <div className="relative mb-1 p-2 rounded-lg hover:bg-gray-100/10 dark:hover:bg-white/5 transition-all">
                              {file.type === 'directory' ? (
                                <svg width="64" height="64" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
                                  <path d="M8 20C8 17.7909 9.79086 16 12 16H24L28 20H52C54.2091 20 56 21.7909 56 24V48C56 50.2091 54.2091 52 52 52H12C9.79086 52 8 50.2091 8 48V20Z"
                                        className="fill-blue-500 dark:fill-blueHighlight"/>
                                  <path d="M8 18C8 15.7909 9.79086 14 12 14H22L26 18H52C54.2091 18 56 19.7909 56 22V24H8V18Z"
                                        className="fill-blue-600 dark:fill-blue-400"/>
                                </svg>
                              ) : file.type === 'video' || file.metadata.isVideoFrame ? (
                                <div className="w-24 h-24 rounded-lg overflow-hidden bg-gray-200 dark:bg-gray-700 shadow-sm">
                                  <NativeVideoThumbnail file={file} />
                                </div>
                              ) : file.type === 'image' ? (
                                <div className="w-24 h-24 rounded-lg overflow-hidden bg-gray-200 dark:bg-gray-700 shadow-sm">
                                  <ImagePreview file={file} />
                                </div>
                              ) : (
                                <div className="w-24 h-24 rounded-lg bg-gray-100 dark:bg-gray-700 flex items-center justify-center">
                                  <FileText className="h-12 w-12 text-gray-400 dark:text-gray-500" />
                                </div>
                              )}

                              {/* Status badges */}
                              {indexingPaths?.has(file.path) && (
                                <div className="absolute -bottom-1 -right-1 bg-blue-500 text-white text-[8px] font-bold px-1.5 py-0.5 rounded-full">
                                  ...
                                </div>
                              )}

                              {/* Drive status */}
                              {file.metadata.driveUuid && (
                                <div className="absolute -top-1 -right-1">
                                  <DriveStatusIndicator
                                    driveUuid={file.metadata.driveUuid}
                                    driveName={file.metadata.driveName}
                                    driveCustomName={file.metadata.driveCustomName}
                                    drivePhysicalLocation={file.metadata.drivePhysicalLocation}
                                    driveStatus={file.metadata.driveStatus}
                                    size="sm"
                                  />
                                </div>
                              )}
                            </div>

                            {/* Filename and info */}
                            <div className="text-center w-full px-1">
                              <p className="text-xs dark:text-gray-200 text-gray-700 font-normal truncate">
                                {file.name}
                              </p>
                              <p className="text-[10px] dark:text-gray-400 text-gray-500 mt-0.5">
                                {file.metadata.size ? formatFileSize(file.metadata.size) :
                                 file.type === 'directory' ? 'Folder' : 'File'}
                              </p>
                            </div>
                          </div>
                        </PreviewContextMenu>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>

          {/* End of results indicator */}
          {!hasMoreFiles && files.length > 0 && (
            <div className="flex justify-center py-8">
              <span className="dark:text-customGray text-gray-500 text-sm">
                {files.length === 1 ? '1 file' : `${files.length} files`} loaded
              </span>
            </div>
          )}
        </div>
      </div>
    );
  });
