import React, {
  useState,
  useEffect,
  useCallback,
} from "react";
import { MediaFile } from "./MediaPreview/types";
import { PreviewContainer, PreviewActions } from "./MediaPreview/PreviewContainer";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FileItem } from "./FileTree";
import { cn } from "../lib/utils";
import { ReferenceImagePanel } from "./ReferenceImagePanel";
import { ReferenceImageData } from "./SearchBar";
import {
  isSupportedImageExtension,
  isSupportedVideoExtension,
} from "../constants";
import { SearchBar } from "./SearchBar";
import { Info } from "lucide-react";
import { SearchOptions, SemanticFileTypeFilter, SearchType } from "../hooks/useSearch";

interface SearchState {
  query: string;
  results: any[];
  isSearching: boolean;
  type: "text" | "visual" | "tag" | "recent";
  isSearchMode: boolean;
}

interface PreviewAreaProps {
  selectedFile: FileItem | null;
  selectedCollection: string;
  indexingPaths: Set<string>;
  transcribingPaths: Set<string>;
  onAddToIndex: (path: string) => void;
  onTranscribeFile?: (path: string) => void;
  onBulkIndex: (path: string) => void;
  isIndexingDisabled: boolean;
  showReferenceImage: boolean;
  searchState: SearchState;
  totalCount?: number;
  referenceImage?: ReferenceImageData | null;
  onReferenceImageClose?: () => void;
  onRefreshSearch?: () => void; // Callback to refresh search when drives change
}

// Define the structure of indexed files from Rust backend
interface IndexedFile {
  id: string;
  file_path: string;
  metadata: string; // This is a string containing JSON
  score: number;
  status: string;
  created_at: string;
  updated_at: string;
  last_indexed_at: string | null;
  mime_type: string | null;
  parent_file_path: string | null;
  tags: string | null;
  timestamp: number | null;
  timestamp_formatted: string | null;
  frame_number: number | null;
  video_duration: number | null;
  source_type?: string | null;
  chunk_index?: number | null;
  snippet?: string | null;
  // Drive information
  drive_uuid?: string | null;
  drive_name?: string | null;
  drive_custom_name?: string | null;
  drive_physical_location?: string | null;
  drive_status?: string | null;
}

interface PaginatedDirectoryResult {
  items: FileItem[];
  total: number;
  offset: number;
  limit: number;
  has_more: boolean;
}

export const PreviewArea: React.FC<PreviewAreaProps & {
  handleSearch: (query: string, type: SearchType, options?: SearchOptions) => Promise<void>;
  handleFileUpload: (file: File) => void;
  setReferenceImage: (image: ReferenceImageData | null) => void;
  setShowReferenceImage: (show: boolean) => void;
  referenceImage: ReferenceImageData | null;
  showReferenceImage: boolean;
  clearSearch: () => void;
  hasActiveJobs: boolean;
  hasFailedJobs: boolean;
  setShowIndexingInfo: (open: boolean) => void;
}> = ({
  selectedFile,
  selectedCollection,
  indexingPaths,
  transcribingPaths,
  onAddToIndex,
  onTranscribeFile,
  onBulkIndex,
  isIndexingDisabled,
  showReferenceImage,
  searchState,
  totalCount,
  referenceImage,
  onReferenceImageClose,
  onRefreshSearch,
  handleSearch,
  handleFileUpload,
  setReferenceImage,
  setShowReferenceImage,
  showReferenceImage: propShowReferenceImage,
  clearSearch,
  hasActiveJobs,
  hasFailedJobs,
  setShowIndexingInfo,
}) => {
    const [mediaFiles, setMediaFiles] = useState<MediaFile[]>([]);
    const [searchMediaFiles, setSearchMediaFiles] = useState<MediaFile[]>([]);
    const [isLoadingFiles, setIsLoadingFiles] = useState(false);
    const [currentDirectory, setCurrentDirectory] = useState<string | null>(
      null
    );
    const [navigationStack, setNavigationStack] = useState<string[]>([]);
    const [indexedFiles, setIndexedFiles] = useState<IndexedFile[]>([]);
    const [isLoadingIndexedFiles, setIsLoadingIndexedFiles] = useState(false);
    const [error, setError] = useState<string | null>(null);
    // Add pagination state
    const [currentPage, setCurrentPage] = useState(0);
    const [hasMoreFiles, setHasMoreFiles] = useState(true);
    const [isLoadingMore, setIsLoadingMore] = useState(false);
    const PAGE_SIZE = 500; // Load 500 files at a time to get more diverse folders

    // Directory pagination
    const [directoryPage, setDirectoryPage] = useState(0);
    const [hasMoreDirectoryFiles, setHasMoreDirectoryFiles] = useState(false);
    const [isLoadingMoreDirectoryFiles, setIsLoadingMoreDirectoryFiles] = useState(false);
    const [directoryTotalCount, setDirectoryTotalCount] = useState(0);
    const DIRECTORY_PAGE_SIZE = 300;

    // Search result pagination
    const [searchPage, setSearchPage] = useState(1);
    const SEARCH_PAGE_SIZE = 60;

    // Add state for viewMode and fileTypeFilter
    const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
    const [fileTypeFilter, setFileTypeFilter] = useState<string>('all');
    const semanticFileTypeFilter: SemanticFileTypeFilter =
      fileTypeFilter === "image" ||
      fileTypeFilter === "video" ||
      fileTypeFilter === "audio" ||
      fileTypeFilter === "document"
        ? fileTypeFilter
        : "all";

    const { query: searchQuery, results: searchResults, isSearching, type: searchType, isSearchMode } = searchState;



    // Add a helper function to get the basename from a path
    const getBaseName = (filepath: string): string => {
      // Handle both Unix and Windows paths
      const normalizedPath = filepath.replace(/\\/g, "/");
      const parts = normalizedPath.split("/");
      const basename = parts[parts.length - 1] || "";
      // Decode URL-encoded paths (like %2F -> /, %20 -> space)
      try {
        return decodeURIComponent(basename);
      } catch (e) {
        // If decoding fails, return the original basename
        return basename;
      }
    };

    const getParentDirectory = (filepath?: string | null): string | null => {
      if (!filepath) return null;
      const normalized = filepath.replace(/\\/g, "/");
      const lastSlash = normalized.lastIndexOf("/");
      if (lastSlash <= 0) return null;
      return normalized.slice(0, lastSlash);
    };

    const normalizeDisplayPath = (path: string): string => {
      let normalized = path.replace("asset://localhost/", "");
      try {
        normalized = decodeURIComponent(normalized);
      } catch {
        // Keep raw path when decoding fails.
      }
      return normalized;
    };

    // File type detection helper
    const getFileType = (filename: string): MediaFile["type"] => {
      const ext = filename.split(".").pop()?.toLowerCase() || "";
      if (isSupportedVideoExtension(ext)) return "video";
      if (isSupportedImageExtension(ext)) return "image";
      if (["mp3", "wav", "ogg", "flac", "aac"].includes(ext)) return "audio";
      if (["pdf", "doc", "docx", "txt", "rtf", "md"].includes(ext))
        return "document";
      if (ext === "") return "directory";
      return "document"; // Default to document type for unknown extensions
    };

    // Directory loading with pagination
    const loadFiles = async (path: string, reset: boolean = true) => {
      if (!path) return;

      // Normalize the path by removing asset:// prefix if present
      const normalizedPath = path.replace("asset://localhost/", "");
      const offset = reset ? 0 : directoryPage * DIRECTORY_PAGE_SIZE;

      try {
        if (reset) {
          setDirectoryPage(0);
          setHasMoreDirectoryFiles(false);
          setDirectoryTotalCount(0);
          setMediaFiles([]);
        } else {
          setIsLoadingMoreDirectoryFiles(true);
        }

        const payload = await invoke<PaginatedDirectoryResult>("list_directory_paginated", {
          path: normalizedPath,
          offset,
          limit: DIRECTORY_PAGE_SIZE,
        });

        const convertedFiles = payload.items.map((f) => ({
          path: f.is_dir ? f.path : convertFileSrc(f.path),
          name: f.name,
          type: f.is_dir ? ("directory" as const) : getFileType(f.name),
          metadata: {
            size: 0,
            modified: new Date().toISOString(),
            isIndexed: false,
          },
        }));

        setMediaFiles((prev) => (reset ? convertedFiles : [...prev, ...convertedFiles]));
        setDirectoryTotalCount(payload.total);
        setHasMoreDirectoryFiles(payload.has_more);
        setDirectoryPage(reset ? 1 : (prev) => prev + 1);
      } catch (error) {
        console.error("Error loading files from path:", normalizedPath, error);
        if (reset) {
          setMediaFiles([]);
          setDirectoryTotalCount(0);
          setHasMoreDirectoryFiles(false);
        }
      } finally {
        setIsLoadingFiles(false);
        setIsLoadingMoreDirectoryFiles(false);
      }
    };

    // Function to load indexed files with pagination
    const loadIndexedFiles = async (reset: boolean = true) => {
      try {
        if (reset) {
          setIsLoadingIndexedFiles(true);
          setCurrentPage(0);
          setHasMoreFiles(true);
          setIndexedFiles([]);
          setMediaFiles([]);
        } else {
          setIsLoadingMore(true);
        }
        setError(null);

        const offset = reset ? 0 : currentPage * PAGE_SIZE;

        // Use paginated grouped files for better performance
        const newFiles = await invoke<IndexedFile[]>(
          "get_indexed_files_grouped_paginated",
          {
            offset,
            limit: PAGE_SIZE,
          }
        );

        // Update state based on whether this is a reset or load more
        if (reset) {
          setIndexedFiles(newFiles);
          setCurrentPage(1);
        } else {
          setIndexedFiles((prev) => [...prev, ...newFiles]);
          setCurrentPage((prev) => prev + 1);
        }

        // Check if there are more files to load
        setHasMoreFiles(newFiles.length === PAGE_SIZE);

        // If we're viewing the indexed collection, show these files
        if (selectedCollection === "indexed") {
          const allFiles = reset ? newFiles : [...indexedFiles, ...newFiles];
          const mediaFilesFromIndexed = processFilesForDisplay(allFiles);
          setMediaFiles(mediaFilesFromIndexed);
        }
      } catch (error) {
        console.error("Failed to load indexed files:", error);
        setError("Failed to load indexed files");
        if (reset) {
          setIndexedFiles([]);
          setMediaFiles([]);
        }
      } finally {
        setIsLoadingIndexedFiles(false);
        setIsLoadingMore(false);
      }
    };

    // Function to load more indexed files
    const loadMoreIndexedFiles = async () => {
      if (!hasMoreFiles || isLoadingMore || isLoadingIndexedFiles) return;
      await loadIndexedFiles(false);
    };

    const loadMoreDirectoryFiles = async () => {
      if (!currentDirectory && !selectedFile?.path) return;
      if (isLoadingMoreDirectoryFiles || isLoadingFiles || !hasMoreDirectoryFiles) return;
      const pathToLoad = currentDirectory || selectedFile?.path;
      if (!pathToLoad) return;
      await loadFiles(pathToLoad, false);
    };

    // Load indexed files when the indexed collection is selected
    useEffect(() => {
      if (selectedCollection === "indexed") {
        setIsLoadingIndexedFiles(true); // Set loading state immediately to prevent empty state flash
        loadIndexedFiles();
      }
    }, [selectedCollection]);

    useEffect(() => {
      if (selectedCollection === "indexed" && totalCount !== undefined) {
        loadIndexedFiles(true);
      }
    }, [totalCount, selectedCollection]);

    const refreshIndexedFiles = async () => {
      await loadIndexedFiles(true);
    };

    // Listen for drive connection/disconnection events to refresh search results
    useEffect(() => {
      const setupDriveEventListeners = async () => {
        const unlistenConnected = await listen('drive_connected', () => {
          // Sync drives to update database with current status
          invoke('sync_drives_to_database').then(() => {
            // If we're currently showing search results, refresh them
            if (searchQuery && isSearchMode && onRefreshSearch) {
              onRefreshSearch();
            }
            // Also refresh indexed files if we're viewing the indexed collection
            if (selectedCollection === "indexed") {
              refreshIndexedFiles();
            }
          }).catch(err => {
            console.warn('Failed to sync drives after connection:', err);
          });
        });

        const unlistenDisconnected = await listen('drive_disconnected', () => {
          // If we're currently showing search results, refresh them to update drive status
          if (searchQuery && isSearchMode && onRefreshSearch) {
            // Small delay to allow drive status to update in database
            setTimeout(() => {
              onRefreshSearch();
            }, 500);
          }
          // Refresh indexed files to update drive status indicators (offline badges, etc.)
          if (selectedCollection === "indexed") {
            setTimeout(() => {
              refreshIndexedFiles();
            }, 500);
          }
        });

        return () => {
          unlistenConnected();
          unlistenDisconnected();
        };
      };

      const cleanup = setupDriveEventListeners();
      return () => {
        cleanup.then(fn => fn());
      };
    }, [searchQuery, isSearchMode, selectedCollection, onRefreshSearch]);


    // **OPTIMIZED: Memoized file processing to avoid unnecessary re-renders**
    const processFilesForDisplay = useCallback(
      (files: any[]): MediaFile[] => {
        let processedFiles = files
          .filter((file) => file && file.file_path) // Filter out invalid items
          .map((file, index) => {
            try {
              // Parse metadata if it's a string
              let parsedMetadata: any = {};
              if (typeof file.metadata === "string") {
                try {
                  parsedMetadata = JSON.parse(file.metadata || "{}");
                } catch (e) {
                  console.error(
                    `Failed to parse metadata for ${file.file_path}:`,
                    e
                  );
                  parsedMetadata = {};
                }
              } else {
                parsedMetadata = file.metadata || {};
              }

              // Get mime type from the file record
              const mimeType = file.mime_type || null;

              // Decode the file path if it's URL-encoded (for disconnected drives)
              let decodedFilePath = file.file_path;
              try {
                // Check if the path contains URL encoding
                if (file.file_path.includes('%')) {
                  decodedFilePath = decodeURIComponent(file.file_path);
                }
              } catch (e) {
                // If decoding fails, use the original path
                decodedFilePath = file.file_path;
              }

              // Generate a filename if needed
              const fileName = getBaseName(decodedFilePath);

              const sourceTypeRaw =
                typeof file.source_type === "string"
                  ? file.source_type
                  : typeof parsedMetadata.source_type === "string"
                    ? parsedMetadata.source_type
                    : null;
              const sourceType = sourceTypeRaw?.toLowerCase() || null;
              const isTranscriptChunk = sourceType === "transcript_chunk";
              const isTextChunk =
                sourceType === "text_chunk" ||
                sourceType === "text_document" ||
                file.chunk_index !== undefined && file.chunk_index !== null;
              const timestampValue =
                typeof file.timestamp === "number"
                  ? file.timestamp
                  : typeof parsedMetadata.time_start_seconds === "number"
                    ? parsedMetadata.time_start_seconds
                    : null;
              const timestampFormatted =
                file.timestamp_formatted ||
                parsedMetadata.timestamp_formatted ||
                parsedMetadata.time_start_formatted ||
                null;
              const resultSnippet =
                file.snippet || parsedMetadata.snippet || null;

              // Check if this is a grouped video (from backend grouping logic).
              const isGroupedVideo = Boolean(
                !isTranscriptChunk &&
                  ((mimeType?.startsWith("video/") &&
                    mimeType !== "video/frame" &&
                    !isTextChunk) ||
                    sourceType === "video" ||
                    (parsedMetadata.frame_count && parsedMetadata.frame_count > 0))
              );

              // Determine if this is a video frame (avoid treating transcript hits as frames).
              const isVideoFrame =
                !isGroupedVideo &&
                !isTranscriptChunk &&
                ((sourceType === "video_frame") ||
                  (file.frame_number !== undefined &&
                    file.frame_number !== null) ||
                  (file.video_duration !== undefined &&
                    file.video_duration !== null) ||
                  file.mime_type === "video/frame" ||
                  (file.id && file.id.includes(":frame:")));

              // Skip reference images
              if (
                file.type === "reference_image" ||
                parsedMetadata.isReferenceImage
              ) {
                return null;
              }

              // Determine the file type
              let finalType: MediaFile["type"] = "document";

              if (isGroupedVideo) {
                finalType = "video"; // Grouped videos should be displayed as videos
              } else if (isVideoFrame) {
                finalType = "image"; // Individual video frames are displayed as images but with special metadata
              } else if (isTranscriptChunk) {
                finalType = "video";
              } else if (mimeType?.startsWith("image/")) {
                finalType = "image";
              } else if (mimeType?.startsWith("video/")) {
                finalType = "video";
              } else if (mimeType?.startsWith("audio/")) {
                finalType = "audio";
              } else {
                // Fall back to extension checking
                const ext = fileName.split(".").pop()?.toLowerCase() || "";
                if (isSupportedImageExtension(ext)) finalType = "image";
                if (isSupportedVideoExtension(ext)) finalType = "video";
                if (["mp3", "wav", "ogg"].includes(ext)) finalType = "audio";
              }

              // Decode parent path if needed
              let decodedParentPath = file.parent_file_path;
              if (decodedParentPath && decodedParentPath.includes('%')) {
                try {
                  decodedParentPath = decodeURIComponent(decodedParentPath);
                } catch (e) {
                  decodedParentPath = file.parent_file_path;
                }
              }

              // Derive parent directory from available sources when parent_file_path is missing.
              const metadataVideoPath =
                parsedMetadata.video_path ||
                parsedMetadata.source_path ||
                parsedMetadata.original_path ||
                null;
              const fallbackParentPath =
                decodedParentPath ||
                getParentDirectory(metadataVideoPath) ||
                getParentDirectory(decodedFilePath);

              // Create the base metadata object
              const baseMetadata = {
                size: parsedMetadata.fs_size || 0,
                modified: file.updated_at || "",
                created: file.created_at || "",
                lastIndexed: file.last_indexed_at || null,
                mimeType: mimeType,
                parentPath: fallbackParentPath || null,
                tags: file.tags || null,
                isDirectory: false,
                score: file.score || 0,
                isIndexed:
                  file.status === "indexed" || file.last_indexed_at !== null,
                sourceType: sourceType || undefined,
                snippet: resultSnippet,
                chunkIndex: file.chunk_index ?? null,
                isTranscriptHit: isTranscriptChunk,
                // Drive information
                driveUuid: file.drive_uuid || null,
                driveName: file.drive_name || null,
                driveCustomName: file.drive_custom_name || null,
                drivePhysicalLocation: file.drive_physical_location || null,
                driveStatus: file.drive_status || null,
              };

              // Create the media file object
              const mediaFile: MediaFile = {
                path: convertFileSrc(decodedFilePath),
                name: fileName,
                type: finalType,
                originalIndex: index,
                score: file.score,
                metadata: baseMetadata,
              };

              // Add grouped video-specific properties
              if (isGroupedVideo) {
                mediaFile.metadata = {
                  ...baseMetadata,
                  frameCount: parsedMetadata.frame_count || 0,
                  bestMatchTimestamp: parsedMetadata.best_match_timestamp || 0,
                  bestMatchFrame: parsedMetadata.best_match_frame_id || "",
                  videoDuration: parsedMetadata.video_duration || 0,
                  isGroupedVideo: true,
                  sourceType: "grouped_video",
                };
              }
              // Add video frame specific properties for individual frames
              else if (isVideoFrame) {
                mediaFile.metadata = {
                  ...baseMetadata,
                  isVideoFrame: true,
                  timestamp: timestampValue,
                  timestampFormatted,
                  frameNumber: file.frame_number,
                  videoDuration: file.video_duration,
                  parentPath:
                    fallbackParentPath,
                  sourceType: "video_frame",
                };

                // For video frames, also add the timestamp info to the name for display
                if (timestampFormatted) {
                  mediaFile.name = `${fileName} @ ${timestampFormatted}`;
                }
              } else if (isTranscriptChunk) {
                mediaFile.metadata = {
                  ...baseMetadata,
                  sourceType: "transcript_chunk",
                  isTranscriptHit: true,
                  timestamp: timestampValue,
                  timestampFormatted,
                  chunkIndex: file.chunk_index ?? null,
                  snippet: resultSnippet,
                  parentPath: fallbackParentPath,
                };
              } else if (isTextChunk) {
                mediaFile.metadata = {
                  ...baseMetadata,
                  sourceType: sourceType || "text_chunk",
                  chunkIndex: file.chunk_index ?? null,
                  snippet: resultSnippet,
                };
              }

              return mediaFile;
            } catch (error) {
              console.error("Error creating MediaFile:", error, file);
              return null;
            }
          })
          .filter((item): item is MediaFile => item !== null);

        // Group video frames by parent video file ONLY when viewing AI Library root (not during search)
        if (selectedCollection === "indexed" && !isSearchMode) {
          const grouped: Record<string, MediaFile> = {};
          const result: MediaFile[] = [];
          processedFiles.forEach((file) => {
            if (file.metadata.isVideoFrame && file.name.includes(" @ ")) {
              const videoFileName = file.name.split(" @ ")[0];
              const groupKey =
                (file.metadata.parentPath || "") + "/" + videoFileName;
              if (!grouped[groupKey]) {
                // Clone the file, update the name, and force type to 'video'
                grouped[groupKey] = {
                  ...file,
                  name: videoFileName,
                  type: "video", // Ensure grouped representative is always type 'video'
                };
              }
            } else {
              result.push(file);
            }
          });
          // Add one representative per video
          Object.values(grouped).forEach((file) => result.push(file));
          processedFiles = result;
        }

        // Remove the reference image logic from here since it's now handled in SearchBar
        return processedFiles;
      },
      [selectedCollection, isSearchMode, searchQuery]
    );

    // Reset current directory when sidebar selection changes
    useEffect(() => {
      if (selectedFile) {
        const selectedPath = selectedFile.path.replace("asset://localhost/", "");
        const currentPath = currentDirectory || navigationStack[navigationStack.length - 1];

        // Only reset if the selected file path is different from current navigation
        if (selectedPath !== currentPath) {
          setCurrentDirectory(null);
          setNavigationStack([]);
        }
      }
    }, [selectedFile]);

    // Update search media files when search results change (separate from indexed files)
    useEffect(() => {
      if (isSearchMode && searchResults.length > 0) {
        setSearchMediaFiles(processFilesForDisplay(searchResults));
        return;
      }
      if (isSearchMode && searchResults.length === 0 && !isSearching) {
        setSearchMediaFiles([]);
        return;
      }
      // Clear search results when exiting search mode
      if (!isSearchMode) {
        setSearchMediaFiles([]);
      }
    }, [searchResults, isSearchMode, isSearching, processFilesForDisplay]);

    useEffect(() => {
      setSearchPage(1);
    }, [searchQuery, searchType, searchResults.length, fileTypeFilter]);

    useEffect(() => {
      if (!isSearchMode || searchType !== "text") return;
      if (!searchQuery.trim()) return;
      handleSearch(searchQuery, "text", { semanticFileTypeFilter });
    }, [fileTypeFilter]);

    const displayedSearchMediaFiles = searchMediaFiles.slice(
      0,
      searchPage * SEARCH_PAGE_SIZE
    );
    const hasMoreSearchFiles = displayedSearchMediaFiles.length < searchMediaFiles.length;

    const loadMoreSearchFiles = () => {
      if (!hasMoreSearchFiles) return;
      setSearchPage((prev) => prev + 1);
    };

    // Load files when selected file or current directory changes
    useEffect(() => {
      // Skip direct filesystem loading while searching.
      if (isSearchMode) {
        console.log('Skipping file load - showing search results');
        return;
      }

      // In indexed mode, only skip when we're at root. If currentDirectory is set,
      // we intentionally drill into that filesystem directory.
      if (selectedCollection === "indexed" && !currentDirectory) {
        console.log('Skipping file load - showing indexed root');
        return;
      }

      const pathToLoad = currentDirectory || selectedFile?.path;

      if (!pathToLoad) {
        console.log('No path to load, clearing media files');
        setMediaFiles([]);
        return;
      }

      // Normalize the path
      const normalizedPath = pathToLoad.replace("asset://localhost/", "");
      console.log('Loading path:', normalizedPath);

      // Immediately clear files and set loading state to prevent flash of old content
      setMediaFiles([]);
      setIsLoadingFiles(true);

      // Check if it's a single file
      const loadPath = async () => {
        try {
          const isDirectory = await invoke<boolean>("is_directory", {
            path: normalizedPath,
          });

          if (!isDirectory && selectedFile) {
            // Handle single file
            const singleFile: MediaFile = {
              path: convertFileSrc(normalizedPath),
              name: selectedFile.name,
              type: getFileType(selectedFile.name),
              metadata: {
                size: 0,
                modified: new Date().toISOString(),
                isIndexed: false,
              },
            };
            setMediaFiles([singleFile]);
            setIsLoadingFiles(false);
          } else {
            // Handle directory - use simple loading (which will handle loading state)
            await loadFiles(normalizedPath);
          }
        } catch (error) {
          console.error("Error checking path type for:", normalizedPath, error);
          setMediaFiles([]);
          setIsLoadingFiles(false);
        }
      };

      loadPath();
    }, [
      selectedFile,
      currentDirectory,
      selectedCollection,
      isSearchMode,
    ]);

    // Restore indexed root listing when leaving a drilled directory.
    useEffect(() => {
      if (selectedCollection === "indexed" && !isSearchMode && !currentDirectory) {
        setMediaFiles(processFilesForDisplay(indexedFiles));
      }
    }, [selectedCollection, isSearchMode, currentDirectory, indexedFiles, processFilesForDisplay]);

    const handleDirectorySelect = (directory: MediaFile) => {
      // Normalize the directory path by removing asset:// prefix if present
      const normalizedPath = directory.path.replace("asset://localhost/", "");
      const currentNormalizedPath = currentDirectory?.replace(
        "asset://localhost/",
        ""
      );

      // Prevent navigating to the same directory
      if (normalizedPath === currentNormalizedPath) {
        return;
      }

      setCurrentDirectory(normalizedPath);
      setNavigationStack((prev) => {
        // Prevent duplicate entries - only add if not already at the end of stack
        if (prev.length === 0 || prev[prev.length - 1] !== normalizedPath) {
          const newStack = [...prev, normalizedPath];
          return newStack;
        } else {
          return prev;
        }
      });
    };

    const handleNavigateBack = (e?: React.MouseEvent) => {
      e?.preventDefault();
      e?.stopPropagation();

      if (navigationStack.length > 1) {
        const newStack = [...navigationStack];
        newStack.pop(); // Remove current directory
        const previousDirectory = newStack[newStack.length - 1];

        setCurrentDirectory(previousDirectory || null);
        setNavigationStack(newStack);
      } else if (navigationStack.length === 1 || currentDirectory) {
        setCurrentDirectory(null);
        setNavigationStack([]);
      }
    };

    const handleBreadcrumbNavigate = (targetPath: string | null) => {
      if (!targetPath) {
        setCurrentDirectory(null);
        setNavigationStack([]);
        return;
      }

      const normalizedTargetPath = targetPath.replace("asset://localhost/", "");
      const selectedRootPath = selectedFile?.path
        ? selectedFile.path.replace("asset://localhost/", "")
        : null;

      if (selectedRootPath && normalizedTargetPath === selectedRootPath) {
        setCurrentDirectory(null);
        setNavigationStack([]);
        return;
      }

      setCurrentDirectory(normalizedTargetPath);
      setNavigationStack((prev) => {
        const existingIndex = prev.indexOf(normalizedTargetPath);
        if (existingIndex >= 0) {
          return prev.slice(0, existingIndex + 1);
        }
        return [...prev, normalizedTargetPath];
      });
    };

    const getBreadcrumbs = () => {
      if (!currentDirectory && !selectedFile) return null;

      const rawPath = currentDirectory || selectedFile?.path || "";
      const normalizedPath = normalizeDisplayPath(rawPath);
      const parts = normalizedPath.split("/").filter(Boolean);
      const hasLeadingSlash = normalizedPath.startsWith("/");

      const crumbEntries = parts.map((part, index) => {
        const prefix = hasLeadingSlash ? "/" : "";
        const pathValue = `${prefix}${parts.slice(0, index + 1).join("/")}`;
        return {
          label: part,
          path: pathValue,
          isLast: index === parts.length - 1,
        };
      });

      return (
        <div className="flex items-center gap-1 px-4 py-2 bg-white dark:bg-darkBgMid border-b border-gray-200 dark:border-darkBgHighlight text-sm">
          {(navigationStack.length > 0 || currentDirectory) && (
            <button
              onClick={(e) => handleNavigateBack(e)}
              className="text-blue-500 hover:text-blue-700 dark:text-blueHighlight dark:hover:text-customWhite mr-2"
            >
              ← Back
            </button>
          )}

          <button
            onClick={() => handleBreadcrumbNavigate(selectedFile?.path || null)}
            className="text-blue-500 hover:text-blue-700 dark:text-blueHighlight dark:hover:text-customWhite"
          >
            Root
          </button>

          {crumbEntries.length > 0 && (
            <span className="text-gray-500 dark:text-customGray">/</span>
          )}

          {crumbEntries.map((entry) => (
            <React.Fragment key={entry.path}>
              {entry.isLast ? (
                <span className="truncate max-w-[200px] font-medium text-gray-900 dark:text-gray-100">
                  {entry.label}
                </span>
              ) : (
                <button
                  onClick={() => handleBreadcrumbNavigate(entry.path)}
                  className="truncate max-w-[200px] text-blue-500 hover:text-blue-700 dark:text-blueHighlight dark:hover:text-customWhite"
                >
                  {entry.label}
                </button>
              )}
              {!entry.isLast && (
                <span className="text-gray-500 dark:text-gray-400">/</span>
              )}
            </React.Fragment>
          ))}
        </div>
      );
    };

    // Get current directory path for bulk indexing
    const getCurrentDirectoryPath = () => {
      if (selectedCollection === "indexed") return undefined;
      return (
        currentDirectory ||
        (selectedFile?.is_dir ? selectedFile.path : undefined)
      );
    };

    const isIndexedRootMode =
      selectedCollection === "indexed" && !currentDirectory && !isSearchMode;
    const activeOnLoadMore = isIndexedRootMode
      ? loadMoreIndexedFiles
      : loadMoreDirectoryFiles;
    const activeHasMoreFiles = isIndexedRootMode ? hasMoreFiles : hasMoreDirectoryFiles;
    const activeIsLoadingMore = isIndexedRootMode
      ? isLoadingMore
      : isLoadingMoreDirectoryFiles;
    const activeTotalCount = isIndexedRootMode
      ? totalCount
      : directoryTotalCount > 0
        ? directoryTotalCount
        : undefined;

    // **NEW: Clear error when starting new operations**
    useEffect(() => {
      if (isLoadingFiles || isSearching) {
        setError(null);
      }
    }, [isLoadingFiles, isSearching]);

    // Combined header
    return (
      <div className="flex flex-col bg-gray-50" data-tour="preview-area">
        <div className="border-b dark:border-darkBgHighlight border-gray-200">
          <div className="flex flex-wrap items-center gap-4 dark:bg-darkBg px-6 py-3">
            <div className="flex-1 min-w-0">
              <SearchBar
                onSearch={(query, type) =>
                  handleSearch(
                    query,
                    type,
                    type === "text" ? { semanticFileTypeFilter } : undefined
                  )
                }
                onFileUpload={handleFileUpload}
                isSearchDisabled={isIndexingDisabled}
                onReferenceImageChange={setReferenceImage}
                onShowReferenceImageChange={setShowReferenceImage}
                referenceImage={referenceImage}
                showReferenceImage={showReferenceImage}
                onClearSearch={clearSearch}
              />
            </div>
            <PreviewActions
              viewMode={viewMode}
              setViewMode={setViewMode}
              fileTypeFilter={fileTypeFilter}
              setFileTypeFilter={setFileTypeFilter}
              onBulkIndex={onBulkIndex}
              currentDirectoryPath={getCurrentDirectoryPath()}
              isIndexingDisabled={isIndexingDisabled}
              handleBulkIndexClick={() => {
                const path = getCurrentDirectoryPath();
                if (path && onBulkIndex) {
                  onBulkIndex(path);
                }
              }}
            />
            <div className="flex items-center gap-2 flex-shrink-0">
              <button
                className="relative p-2 rounded-lg transition-all duration-300 dark:hover:bg-darkBgHighlight hover:bg-gray-100 dark:hover:text-customBlue dark:text-customGray text-gray-500 hover:text-gray-700"
                onClick={() => setShowIndexingInfo(true)}
              >
                <Info className="h-4 w-4 transition-colors duration-300" />
                {hasActiveJobs && (
                  <div className="absolute -top-1 -right-1 w-3 h-3 dark:bg-blueHighlight bg-blue-500 rounded-full animate-pulse">
                    <div className="absolute inset-0 dark:bg-blueHighlight bg-blue-500 rounded-full animate-ping opacity-75"></div>
                  </div>
                )}
                {hasFailedJobs && !hasActiveJobs && (
                  <div className="absolute -top-1 -right-1 w-3 h-3 dark:bg-customRed bg-red-500 rounded-full"></div>
                )}
              </button>
            </div>
          </div>
        </div>
        {getBreadcrumbs()}

        {/* Empty state for AI Library */}
        {selectedCollection === "indexed" &&
        !isLoadingFiles &&
        !isLoadingIndexedFiles &&
        !isSearching &&
        !searchQuery &&
        mediaFiles.length === 0 &&
        searchMediaFiles.length === 0 &&
        indexedFiles.length === 0 ? (
          <div className="flex flex-1 dark:bg-darkBgMid flex-col items-center justify-center py-24 text-center">
            <div className="text-4xl mb-4">📂</div>
            <h2 className="text-2xl font-semibold dark:text-customWhite text-gray-800 mb-2">
              No files indexed yet
            </h2>
            <p className="text-gray-500 dark:text-customGray mb-6 max-w-md mx-auto">
              Start by indexing a folder or file. Right click a directory and
              use the{" "}
              <span className="font-semibold">
                Index All Files in Directory
              </span>{" "}
              button to begin building your media library.
            </p>
          </div>
        ) : searchQuery && showReferenceImage && referenceImage ? (
          <div
            className="flex w-full h-full relative dark:bg-darkBg"
            data-tour="reference-image"
          >
            <div className="flex-1 flex flex-col">
              <div
                className={cn(
                  "flex flex-col w-full",
                  showReferenceImage ? "pr-[344px]" : ""
                )}
              >
            <PreviewContainer
              key={`search-${searchQuery}-${viewMode}`}
              files={displayedSearchMediaFiles}
              initialViewMode="grid"
              onLoadMore={loadMoreSearchFiles}
              isLoadingFiles={isSearching}
              onDirectorySelect={handleDirectorySelect}
              indexingPaths={indexingPaths}
              transcribingPaths={transcribingPaths}
              onAddToIndex={async (path) => {
                try {
                  await onAddToIndex(path);
                } catch (error) {
                  console.error("Failed to add to index:", error);
                }
              }}
              onTranscribeFile={onTranscribeFile}
              onBulkIndex={onBulkIndex}
              currentDirectoryPath={getCurrentDirectoryPath()}
              isIndexingDisabled={isIndexingDisabled}
              hasMoreFiles={hasMoreSearchFiles}
              isLoadingMore={false}
              isSearchMode={true}
              selectedCollection={selectedCollection}
              viewMode={viewMode}
              setViewMode={setViewMode}
              fileTypeFilter={fileTypeFilter}
              setFileTypeFilter={setFileTypeFilter}
            />
              </div>
            </div>
            <div className="fixed top-24 right-6 z-50">
              <ReferenceImagePanel
                imageUrl={referenceImage.url}
                imageName={referenceImage.name}
                onClose={onReferenceImageClose}
              />
            </div>
          </div>
        ) : searchQuery ? (
          <div
            className={cn(
              "flex flex-col w-full",
              showReferenceImage ? "pr-[344px]" : ""
            )}
          >
            {/* isSearchMode is set to true for all search results, so PreviewContainer will not group by folder or video */}
            <PreviewContainer
              key={`search-${searchQuery}-${viewMode}`}
              files={displayedSearchMediaFiles}
              initialViewMode="grid"
              onLoadMore={loadMoreSearchFiles}
              isLoadingFiles={isSearching}
              onDirectorySelect={handleDirectorySelect}
              indexingPaths={indexingPaths}
              transcribingPaths={transcribingPaths}
              onAddToIndex={async (path) => {
                try {
                  await onAddToIndex(path);
                } catch (error) {
                  console.error("Failed to add to index:", error);
                }
              }}
              onTranscribeFile={onTranscribeFile}
              onBulkIndex={onBulkIndex}
              currentDirectoryPath={getCurrentDirectoryPath()}
              isIndexingDisabled={isIndexingDisabled}
              hasMoreFiles={hasMoreSearchFiles}
              isLoadingMore={false}
              isSearchMode={true}
              selectedCollection={selectedCollection}
              viewMode={viewMode}
              setViewMode={setViewMode}
              fileTypeFilter={fileTypeFilter}
              setFileTypeFilter={setFileTypeFilter}
            />
          </div>
        ) : selectedFile && selectedFile.is_dir && (!selectedFile.path || selectedFile.path.trim() === '') ? (
          // Empty state for unmounted drives
          <div className="flex flex-1 dark:bg-darkBgMid flex-col items-center justify-center py-24 text-center">
            <div className="text-4xl mb-4">💾</div>
            <h2 className="text-2xl font-semibold dark:text-customWhite text-gray-800 mb-2">
              Drive Not Mounted
            </h2>
            <p className="text-gray-500 dark:text-customGray mb-6 max-w-md mx-auto">
              The drive "{selectedFile.name}" is connected but not mounted.
              Please mount the drive in your system to view its contents.
            </p>
          </div>
        ) : (
          <PreviewContainer
            key={`files-${currentDirectory || selectedFile?.path || "none"}-${viewMode}`}
            files={mediaFiles}
            initialViewMode={viewMode}
            onLoadMore={activeOnLoadMore}
            isLoadingFiles={isLoadingFiles || isLoadingIndexedFiles}
            onDirectorySelect={handleDirectorySelect}
            indexingPaths={indexingPaths}
            transcribingPaths={transcribingPaths}
            onAddToIndex={async (path) => {
              try {
                await onAddToIndex(path);
              } catch (error) {
                console.error("Failed to add to index:", error);
              }
            }}
            onTranscribeFile={onTranscribeFile}
            onBulkIndex={onBulkIndex}
            currentDirectoryPath={getCurrentDirectoryPath()}
            isIndexingDisabled={isIndexingDisabled}
            hasMoreFiles={activeHasMoreFiles}
            isLoadingMore={activeIsLoadingMore}
            isSearchMode={false}
            selectedCollection={selectedCollection}
            totalCount={activeTotalCount}
            viewMode={viewMode}
            setViewMode={setViewMode}
            fileTypeFilter={fileTypeFilter}
            setFileTypeFilter={setFileTypeFilter}
          />
        )}
      </div>
    );
};

export default PreviewArea;
