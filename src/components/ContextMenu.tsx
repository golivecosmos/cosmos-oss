import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ContextMenu as BaseContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "../components/ui/context-menu";
import {
  Copy,
  Trash2,
  Star,
  Tag,
  Share2,
  Info,
  FolderPlus,
  FileUp,
  Download,
  Database,
  FolderSearch,
  FileText,
} from "lucide-react";
import { FileItem } from "./FileTree";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { getErrorMessage } from "../utils/errorMessage";
import { normalizeFilePath } from "../lib/utils";

interface WatchedFolder {
  id: string;
  path: string;
  recursive: boolean;
  enabled: boolean;
  auto_transcribe_videos: boolean;
  status: string;
  last_scan_at?: string | null;
  last_event_at?: string | null;
  created_at: string;
  updated_at: string;
}

const normalizeWatchedPath = (path: string): string => {
  return path.replace(/\/+$/, "").toLowerCase();
};

const PURE_AUDIO_EXTENSIONS = new Set(["wav", "mp3", "m4a", "flac", "ogg", "aac", "wma"]);
const TRANSCRIBABLE_EXTENSIONS = new Set([
  ...PURE_AUDIO_EXTENSIONS,
  "mp4",
  "mov",
  "avi",
  "mkv",
  "webm",
]);

interface FileContextMenuProps {
  children: React.ReactNode;
  item: FileItem;
  onAddToFavorites?: (item: FileItem) => void;
  onShare?: (item: FileItem) => void;
  onDelete?: (item: FileItem) => void;
  onAddTags?: (item: FileItem) => void;
  onCopyPath?: (item: FileItem) => void;
  onShowInfo?: (item: FileItem) => void;
  onCreateFolder?: (parentItem: FileItem) => void;
  onUpload?: (parentItem: FileItem) => void;
  onDownload?: (item: FileItem) => void;
  onBulkIndex?: (item: FileItem) => void;
  isIndexingDisabled?: boolean;
}

export function FileContextMenu({
  children,
  item,
  onAddToFavorites,
  onShare,
  onDelete,
  onAddTags,
  onCopyPath,
  onShowInfo,
  onCreateFolder,
  onUpload,
  onDownload,
  onBulkIndex,
  isIndexingDisabled,
}: FileContextMenuProps) {
  const [watchedFolder, setWatchedFolder] = useState<WatchedFolder | null>(null);
  const [isWatchingStateLoading, setIsWatchingStateLoading] = useState(false);
  const normalizedItemPath = useMemo(
    () => normalizeFilePath(item.path),
    [item.path]
  );
  const fileExtension = useMemo(
    () => item.name.split(".").pop()?.toLowerCase() ?? "",
    [item.name]
  );
  const isAudioOnlyFile = !item.is_dir && PURE_AUDIO_EXTENSIONS.has(fileExtension);
  const isTranscribableFile = !item.is_dir && TRANSCRIBABLE_EXTENSIONS.has(fileExtension);

  const refreshWatchedFolderState = useCallback(async () => {
    if (!item.is_dir) {
      setWatchedFolder(null);
      return;
    }

    setIsWatchingStateLoading(true);
    try {
      const folders = await invoke<WatchedFolder[]>("list_watched_folders");
      const targetPath = normalizeWatchedPath(normalizedItemPath);
      const matchedFolder =
        folders.find((folder) => normalizeWatchedPath(folder.path) === targetPath) || null;
      setWatchedFolder(matchedFolder);
    } catch {
      setWatchedFolder(null);
    } finally {
      setIsWatchingStateLoading(false);
    }
  }, [item.is_dir, normalizedItemPath]);

  useEffect(() => {
    refreshWatchedFolderState();
  }, [refreshWatchedFolderState]);

  const handleIndexFile = async () => {
    try {
      await invoke("index_file", {
        path: item.path,
        name: item.name,
        isDirectory: item.is_dir,
      });
      toast.success("Added file to search index queue");
    } catch (error) {
      console.error("Failed to index file:", error);
      toast.error(`Failed to index file: ${getErrorMessage(error)}`);
    }
  };

  const handleTranscribeFile = async () => {
    try {
      await invoke("transcribe_file", {
        path: normalizedItemPath,
      });
      toast.success("Added transcription job to queue");
    } catch (error) {
      console.error("Failed to transcribe file:", error);
      toast.error(`Failed to transcribe file: ${getErrorMessage(error)}`);
    }
  };

  const handleWatchFolder = async () => {
    try {
      const folder = await invoke<WatchedFolder>("add_watched_folder", {
        path: normalizedItemPath,
        recursive: true,
        autoTranscribeVideos: true,
      });
      setWatchedFolder(folder);
      toast.success("Folder is now watched");
    } catch (error) {
      toast.error(`Failed to watch folder: ${getErrorMessage(error)}`);
    }
  };

  const handleSetWatchingEnabled = async (enabled: boolean) => {
    if (!watchedFolder) return;
    try {
      await invoke("set_watched_folder_enabled", {
        folderId: watchedFolder.id,
        enabled,
      });
      setWatchedFolder((prev) =>
        prev
          ? {
              ...prev,
              enabled,
              status: enabled ? "watching" : "paused",
            }
          : prev
      );
      toast.success(enabled ? "Watching resumed" : "Watching paused");
    } catch (error) {
      toast.error(`Failed to update watch state: ${getErrorMessage(error)}`);
    }
  };

  const handleScanNow = async () => {
    if (!watchedFolder) return;
    try {
      await invoke("trigger_watched_folder_backfill", {
        folderId: watchedFolder.id,
      });
      toast.success("Watched folder scan started");
    } catch (error) {
      toast.error(`Failed to start scan: ${getErrorMessage(error)}`);
    }
  };

  const handleStopWatching = async () => {
    if (!watchedFolder) return;
    try {
      await invoke("remove_watched_folder", { folderId: watchedFolder.id });
      setWatchedFolder(null);
      toast.success("Stopped watching folder");
    } catch (error) {
      toast.error(`Failed to stop watching: ${getErrorMessage(error)}`);
    }
  };

  return (
    <BaseContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        {!item.is_dir && (
          <>
            {!isAudioOnlyFile &&
              (isIndexingDisabled ? (
                <ContextMenuItem disabled>
                  <Database className="mr-2 h-4 w-4" />
                  Add to Index
                </ContextMenuItem>
              ) : (
                <ContextMenuItem onClick={handleIndexFile}>
                  <Database className="mr-2 h-4 w-4" />
                  Add to Index
                </ContextMenuItem>
              ))}

            {isTranscribableFile && (
              <ContextMenuItem onClick={handleTranscribeFile}>
                <FileText className="mr-2 h-4 w-4" />
                Transcribe
              </ContextMenuItem>
            )}

            <ContextMenuSeparator />
          </>
        )}
        {item.is_dir && (
          <>
            {/* Directory indexing option */}
            {onBulkIndex && (
              <>
                {isIndexingDisabled ? (
                  <ContextMenuItem disabled>
                    <FolderSearch className="mr-2 h-4 w-4" />
                    Index Directory
                  </ContextMenuItem>
                ) : (
                  <ContextMenuItem onClick={() => onBulkIndex(item)}>
                    <FolderSearch className="mr-2 h-4 w-4" />
                    Index Directory
                  </ContextMenuItem>
                )}
              </>
            )}

            <ContextMenuItem
              onClick={handleWatchFolder}
              disabled={isWatchingStateLoading || !!watchedFolder}
            >
              <FolderSearch className="mr-2 h-4 w-4" />
              Watch Folder
            </ContextMenuItem>

            {watchedFolder && (
              <>
                <ContextMenuItem onClick={() => handleSetWatchingEnabled(!watchedFolder.enabled)}>
                  <FolderSearch className="mr-2 h-4 w-4" />
                  {watchedFolder.enabled ? "Pause Watching" : "Resume Watching"}
                </ContextMenuItem>
                <ContextMenuItem onClick={handleScanNow}>
                  <FolderSearch className="mr-2 h-4 w-4" />
                  Scan Watched Folder Now
                </ContextMenuItem>
                <ContextMenuItem onClick={handleStopWatching}>
                  <Trash2 className="mr-2 h-4 w-4" />
                  Stop Watching Folder
                </ContextMenuItem>
              </>
            )}

            <ContextMenuSeparator />
          </>
        )}
        {onAddToFavorites && (
          <ContextMenuItem onClick={() => onAddToFavorites(item)}>
            <Star className="mr-2 h-4 w-4" />
            Add to Favorites
          </ContextMenuItem>
        )}
        {onShare && (
          <ContextMenuItem onClick={() => onShare(item)}>
            <Share2 className="mr-2 h-4 w-4" />
            Share
          </ContextMenuItem>
        )}
        {onAddTags && (
          <ContextMenuItem onClick={() => onAddTags(item)}>
            <Tag className="mr-2 h-4 w-4" />
            Add Tags
          </ContextMenuItem>
        )}
        {onCopyPath && (
          <ContextMenuItem onClick={() => onCopyPath(item)}>
            <Copy className="mr-2 h-4 w-4" />
            Copy Path
          </ContextMenuItem>
        )}
        {onShowInfo && (
          <ContextMenuItem onClick={() => onShowInfo(item)}>
            <Info className="mr-2 h-4 w-4" />
            Show Info
          </ContextMenuItem>
        )}
        {onCreateFolder && item.is_dir && (
          <ContextMenuItem onClick={() => onCreateFolder(item)}>
            <FolderPlus className="mr-2 h-4 w-4" />
            New Folder
          </ContextMenuItem>
        )}
        {onUpload && item.is_dir && (
          <ContextMenuItem onClick={() => onUpload(item)}>
            <FileUp className="mr-2 h-4 w-4" />
            Upload Files
          </ContextMenuItem>
        )}
        {onDownload && (
          <ContextMenuItem onClick={() => onDownload(item)}>
            <Download className="mr-2 h-4 w-4" />
            Download
          </ContextMenuItem>
        )}
        {onDelete && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem
              onClick={() => onDelete(item)}
              className="text-red-600 dark:text-customRed dark:focus:text-redHighlight focus:text-red-600"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
    </BaseContextMenu>
  );
}
