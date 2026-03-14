import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileText } from "lucide-react";

import { MediaFile } from "./types";
import { ImagePreview } from "./ImagePreview";
import { NativeVideoThumbnail } from "./NativeVideoThumbnail";
import { PreviewContextMenu } from "./PreviewContextMenu";
import { formatFileSize, normalizeFilePath } from "../../lib/utils";
import { DriveStatusIndicator } from "../DriveStatusIndicator";
import { OfflineDriveCard } from "../OfflineDriveCard";

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

interface FilePreviewPayload {
  content: string;
}

const TEXT_PREVIEW_BYTES = 4096;
const TEXT_EXTENSIONS = new Set([
  "txt",
  "md",
  "markdown",
  "json",
  "js",
  "jsx",
  "ts",
  "tsx",
  "css",
  "html",
  "htm",
  "xml",
  "csv",
  "log",
  "yml",
  "yaml",
  "toml",
  "ini",
  "conf",
  "py",
  "rb",
  "java",
  "cpp",
  "c",
  "h",
  "rs",
  "go",
  "php",
  "sql",
]);

const textPreviewCache = new Map<string, string | null>();

const getItemsPerRow = (width: number) => {
  if (width >= 1536) return 10;
  if (width >= 1280) return 8;
  if (width >= 1024) return 6;
  if (width >= 768) return 5;
  if (width >= 640) return 4;
  return 3;
};

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

  if (file.metadata.driveUuid) {
    const driveName = file.metadata.driveCustomName || file.metadata.driveName;
    tooltipItems.push(`Drive: ${driveName} (${file.metadata.driveStatus})`);
    if (file.metadata.drivePhysicalLocation) {
      tooltipItems.push(`Location: ${file.metadata.drivePhysicalLocation}`);
    }
  }

  if (file.metadata.isVideoFrame && file.metadata.timestampFormatted) {
    tooltipItems.push(`Timestamp: ${file.metadata.timestampFormatted}`);
  }

  if (file.metadata.score && file.metadata.score > 0) {
    tooltipItems.push(`Match score: ${(file.metadata.score * 100).toFixed(1)}%`);
  }

  return tooltipItems.join("\n");
};

function getFileExtension(fileName: string): string {
  return fileName.split(".").pop()?.toLowerCase() || "";
}

function buildTextSnippet(ext: string, content: string): string {
  let value = content.replace(/\u0000/g, " ");

  if (ext === "html" || ext === "htm") {
    value = value
      .replace(/<script[\s\S]*?<\/script>/gi, " ")
      .replace(/<style[\s\S]*?<\/style>/gi, " ")
      .replace(/<[^>]+>/g, " ");
  }

  if (ext === "json") {
    try {
      value = JSON.stringify(JSON.parse(value), null, 2);
    } catch {
      // Keep raw content when parsing fails.
    }
  }

  const lines = value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .slice(0, 7);

  if (lines.length === 0) return "No text preview";

  const snippet = lines.join("\n");
  return snippet.length > 320 ? `${snippet.slice(0, 320)}...` : snippet;
}

const FileGridPreview = React.memo(({ file }: { file: MediaFile }) => {
  const ext = getFileExtension(file.name);
  const isTextLike = TEXT_EXTENSIONS.has(ext);
  const normalizedPath = normalizeFilePath(file.path);
  const [snippet, setSnippet] = useState<string | null>(
    () => textPreviewCache.get(file.path) ?? null
  );

  useEffect(() => {
    if (!isTextLike) {
      setSnippet(null);
      return;
    }

    let cancelled = false;
    const cached = textPreviewCache.get(file.path);
    if (cached !== undefined) {
      setSnippet(cached);
      return;
    }

    const loadPreview = async () => {
      try {
        const payload = await invoke<FilePreviewPayload>("read_file_preview", {
          path: normalizedPath,
          maxBytes: TEXT_PREVIEW_BYTES,
        });
        if (cancelled) return;
        const value = buildTextSnippet(ext, payload.content);
        textPreviewCache.set(file.path, value);
        setSnippet(value);
      } catch {
        if (cancelled) return;
        textPreviewCache.set(file.path, null);
        setSnippet(null);
      }
    };

    loadPreview();
    return () => {
      cancelled = true;
    };
  }, [ext, file.path, isTextLike, normalizedPath]);

  if (snippet) {
    return (
      <div className="w-24 h-24 rounded-lg bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 overflow-hidden p-2">
        <pre className="font-mono text-[9px] leading-3 whitespace-pre-wrap break-words text-gray-700 dark:text-gray-200 max-h-full overflow-hidden">
          {snippet}
        </pre>
      </div>
    );
  }

  return (
    <div className="w-24 h-24 rounded-lg bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 overflow-hidden p-2">
      <div className="h-full w-full rounded-md border border-gray-200/70 dark:border-gray-500/60 bg-white dark:bg-gray-800 px-2 py-1.5">
        {ext === "pdf" ? (
          <>
            <div className="text-[8px] font-semibold uppercase tracking-wide text-red-600 dark:text-red-300 truncate">
              PDF
            </div>
            <div className="mt-1.5 space-y-1">
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600" />
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600 w-4/5" />
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600 w-3/4" />
            </div>
            <div className="mt-2 flex items-center justify-center">
              <FileText className="h-6 w-6 text-red-400 dark:text-red-300" />
            </div>
          </>
        ) : (
          <>
            <div className="text-[8px] font-semibold uppercase tracking-wide text-blue-600 dark:text-blue-300 truncate">
              {ext || file.type}
            </div>
            <div className="mt-1.5 space-y-1">
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600" />
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600 w-5/6" />
              <div className="h-1 rounded bg-gray-200 dark:bg-gray-600 w-3/4" />
            </div>
            <div className="mt-2 flex items-center justify-center">
              <FileText className="h-6 w-6 text-gray-400 dark:text-gray-500" />
            </div>
          </>
        )}
      </div>
    </div>
  );
});

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
    const [itemsPerRow, setItemsPerRow] = useState(8);

    const sortedFiles = [...files].sort((a, b) => {
      return (a.originalIndex ?? 0) - (b.originalIndex ?? 0);
    });

    useEffect(() => {
      if (!containerRef.current) return;

      const updateItemsPerRow = () => {
        if (!containerRef.current) return;
        setItemsPerRow(getItemsPerRow(containerRef.current.offsetWidth));
      };

      updateItemsPerRow();
      const observer = new ResizeObserver(updateItemsPerRow);
      observer.observe(containerRef.current);
      window.addEventListener("resize", updateItemsPerRow);

      return () => {
        observer.disconnect();
        window.removeEventListener("resize", updateItemsPerRow);
      };
    }, []);

    if (isLoading && files.length === 0) {
      return (
        <div className="p-6">
          <div
            className="grid gap-8"
            style={{ gridTemplateColumns: `repeat(${itemsPerRow}, minmax(0, 1fr))` }}
          >
            {Array.from({ length: itemsPerRow * 2 }).map((_, i) => (
              <div key={i} className="flex flex-col items-center w-full max-w-[140px] mx-auto">
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
          <div
            className="grid gap-8"
            style={{ gridTemplateColumns: `repeat(${itemsPerRow}, minmax(0, 1fr))` }}
          >
            {sortedFiles.map((file, index) => {
              const isOfflineDrive = file.metadata.driveStatus === "disconnected";

              if (isOfflineDrive) {
                return (
                  <div key={`${file.path}-${index}`} className="w-full max-w-[140px] mx-auto">
                    <OfflineDriveCard
                      file={{
                        id: file.path,
                        file_path: file.path.replace("asset://localhost/", ""),
                        metadata: file.metadata.parentPath
                          ? JSON.stringify({ fs_size: file.metadata.size })
                          : "{}",
                        score: file.metadata.score || 0,
                        drive_uuid: file.metadata.driveUuid,
                        drive_name: file.metadata.driveName,
                        drive_custom_name: file.metadata.driveCustomName,
                        drive_physical_location: file.metadata.drivePhysicalLocation,
                        drive_status: file.metadata.driveStatus,
                        mime_type: file.metadata.mimeType,
                        created_at: file.metadata.created || "",
                        timestamp: file.metadata.timestamp,
                        video_duration: file.metadata.videoDuration,
                      }}
                      onClick={() => {
                        alert(
                          `Please connect the drive "${file.metadata.driveCustomName || file.metadata.driveName}" to access this file.`
                        );
                      }}
                    />
                  </div>
                );
              }

              return (
                <PreviewContextMenu
                  key={`${file.path}-${index}`}
                  file={file}
                  onAddToIndex={onAddToIndex}
                  onTranscribeFile={onTranscribeFile}
                  indexingPaths={indexingPaths}
                  isFromSearch={file.metadata.score !== undefined && file.metadata.score > 0}
                >
                  <div
                    className="group flex w-full max-w-[140px] mx-auto flex-col items-center cursor-pointer transition-all duration-150 opacity-90 hover:opacity-100"
                    onClick={() => onFileSelect(file)}
                    title={file.type === "document" ? undefined : getTooltipContent(file)}
                  >
                    <div className="relative mb-1 p-2 rounded-lg hover:bg-gray-100/10 dark:hover:bg-white/5 transition-all">
                      {file.type === "directory" ? (
                        <svg width="64" height="64" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
                          <path
                            d="M8 20C8 17.7909 9.79086 16 12 16H24L28 20H52C54.2091 20 56 21.7909 56 24V48C56 50.2091 54.2091 52 52 52H12C9.79086 52 8 50.2091 8 48V20Z"
                            className="fill-blue-500 dark:fill-blueHighlight"
                          />
                          <path
                            d="M8 18C8 15.7909 9.79086 14 12 14H22L26 18H52C54.2091 18 56 19.7909 56 22V24H8V18Z"
                            className="fill-blue-600 dark:fill-blue-400"
                          />
                        </svg>
                      ) : file.type === "video" || file.metadata.isVideoFrame ? (
                        <div className="w-24 h-24 rounded-lg overflow-hidden bg-gray-200 dark:bg-gray-700 shadow-sm">
                          <NativeVideoThumbnail file={file} />
                        </div>
                      ) : file.type === "image" ? (
                        <div className="w-24 h-24 rounded-lg overflow-hidden bg-gray-200 dark:bg-gray-700 shadow-sm">
                          <ImagePreview file={file} />
                        </div>
                      ) : (
                        <FileGridPreview file={file} />
                      )}

                      {indexingPaths?.has(file.path) && (
                        <div className="absolute -bottom-1 -right-1 bg-blue-500 text-white text-[8px] font-bold px-1.5 py-0.5 rounded-full">
                          ...
                        </div>
                      )}

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

                    <div className="text-center w-full px-1">
                      <p className="text-xs dark:text-gray-200 text-gray-700 font-normal truncate">
                        {file.name}
                      </p>
                      <p className="text-[10px] dark:text-gray-400 text-gray-500 mt-0.5">
                        {file.metadata.size
                          ? formatFileSize(file.metadata.size)
                          : file.type === "directory"
                            ? "Folder"
                            : "File"}
                      </p>
                    </div>
                  </div>
                </PreviewContextMenu>
              );
            })}
          </div>

          {!hasMoreFiles && files.length > 0 && (
            <div className="flex justify-center py-8">
              <span className="dark:text-customGray text-gray-500 text-sm">
                {files.length === 1 ? "1 file" : `${files.length} files`} loaded
              </span>
            </div>
          )}
        </div>
      </div>
    );
  }
);
