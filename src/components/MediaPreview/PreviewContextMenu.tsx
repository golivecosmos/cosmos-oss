import React, { useState, useEffect } from 'react';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '../ui/context-menu';
import { MediaFile } from './types';
import {
  FileText,
  Copy,
  FolderOpen,
  ExternalLink,
  Info,
  BookOpen,
  Video,
  FolderSearch,
  Trash2,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { normalizeFilePath } from '../../lib/utils';
import { toast } from 'sonner';
import { useLocation, useNavigate } from 'react-router-dom';

// Helper function to check if a file can be transcribed (audio or video)
const isTranscribableFile = (fileName: string): boolean => {
  const transcribableExtensions = ['wav', 'mp3', 'mp4', 'm4a', 'flac', 'ogg', 'mov', 'avi', 'mkv', 'webm'];
  const ext = fileName.toLowerCase().split('.').pop();
  return ext ? transcribableExtensions.includes(ext) : false;
};

interface PreviewContextMenuProps {
  file: MediaFile;
  children: React.ReactNode;
  onAddToIndex?: (path: string) => void;
  onTranscribeFile?: (path: string) => void;
  onViewTranscription?: (path: string) => void;
  indexingPaths?: Set<string>;
  transcribingPaths?: Set<string>;
  isFromSearch?: boolean; // New prop to indicate if this is from search results
}

interface WatchedFolder {
  id: string;
  path: string;
  enabled: boolean;
}

const normalizeWatchedPath = (path: string): string => {
  return path.replace(/\/+$/, "").toLowerCase();
};

export function PreviewContextMenu({
  file,
  children,
  onAddToIndex,
  onTranscribeFile,
  onViewTranscription,
  indexingPaths,
  transcribingPaths,
  isFromSearch = false,
}: PreviewContextMenuProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const [isVideoInStudio, setIsVideoInStudio] = useState(false);
  const [isCheckingStudio, setIsCheckingStudio] = useState(false);
  const [watchedFolder, setWatchedFolder] = useState<WatchedFolder | null>(null);
  const [isWatchStateLoading, setIsWatchStateLoading] = useState(false);

  const toFilesystemPath = (input: string): string => {
    let normalized = input;

    if (normalized.startsWith("asset://localhost")) {
      normalized = normalized.slice("asset://localhost".length);
    } else if (normalized.startsWith("asset://")) {
      normalized = normalized.slice("asset://".length);
    } else if (normalized.startsWith("file://")) {
      normalized = normalized.slice("file://".length);
    }

    if (normalized.startsWith("//")) {
      normalized = normalized.replace(/^\/+/, "/");
    }

    try {
      normalized = decodeURIComponent(normalized);
    } catch {
      // Keep raw path when decode fails.
    }

    const isWindowsPath = /^[a-zA-Z]:[\\/]/.test(normalized);
    if (!isWindowsPath && normalized && !normalized.startsWith("/")) {
      normalized = `/${normalized}`;
    }

    return normalized;
  };

  const isIndexing = indexingPaths?.has(file.path);
  const isTranscribing = transcribingPaths?.has(normalizeFilePath(file.path));
  const isAlreadyIndexed = file.metadata.isIndexed || isFromSearch;

  useEffect(() => {
    const checkVideoInStudio = async () => {
      if (file.type === 'video') {
        setIsCheckingStudio(true);
        try {
          const cleanPath = normalizeFilePath(file.path);
          const inStudio = await invoke<boolean>('is_video_in_studio', { videoPath: cleanPath });
          setIsVideoInStudio(inStudio);
        } catch (error) {
          console.error('Failed to check if video is in Studio:', error);
          setIsVideoInStudio(false);
        } finally {
          setIsCheckingStudio(false);
        }
      }
    };

    checkVideoInStudio();
  }, [file.path, file.type]);

  useEffect(() => {
    const loadWatchState = async () => {
      if (file.type !== 'directory') {
        setWatchedFolder(null);
        return;
      }

      setIsWatchStateLoading(true);
      try {
        const targetPath = normalizeFilePath(file.path);
        const folders = await invoke<WatchedFolder[]>('list_watched_folders');
        const normalizedTargetPath = normalizeWatchedPath(targetPath);
        const match =
          folders.find((folder) => normalizeWatchedPath(folder.path) === normalizedTargetPath) ||
          null;
        setWatchedFolder(match);
      } catch {
        setWatchedFolder(null);
      } finally {
        setIsWatchStateLoading(false);
      }
    };

    loadWatchState();
  }, [file.path, file.type]);

  const handleCopyPath = async () => {
    const cleanPath = normalizeFilePath(file.path);
    try {
      await invoke('copy_to_clipboard', { text: cleanPath });
    } catch (error) {
      console.error('Failed to copy path:', error);
      // Fallback to browser clipboard API
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(cleanPath);
      }
    }
  };

  const handleRevealInFinder = async () => {
    const cleanPath = normalizeFilePath(file.path);
    try {
      await invoke('show_in_file_manager', { path: cleanPath });
    } catch (error) {
      console.error('Failed to reveal in finder:', error);
      alert(`Failed to show file in file manager: ${error}`);
    }
  };

  const handleOpenWithDefault = async () => {
    const cleanPath = normalizeFilePath(file.path);
    try {
      await invoke('open_with_default_app', { path: cleanPath });
    } catch (error) {
      console.error('Failed to open with default app:', error);
      alert(`Failed to open file with default application: ${error}`);
    }
  };


  const handleViewTranscription = async () => {
    if (!onViewTranscription) return;
    const cleanPath = normalizeFilePath(file.path);
    onViewTranscription(cleanPath);
  };

  const handleTranscribe = () => {
    if (!onTranscribeFile) return;
    onTranscribeFile(normalizeFilePath(file.path));
  };

  const handleSendToStudio = async () => {
    const cleanPath = toFilesystemPath(file.path);
    const params = new URLSearchParams();
    params.set("path", cleanPath);

    const currentRoute = `${location.pathname}${location.search}`;
    let parentPathFromMetadata: string | null = null;
    if (file.metadata.parentPath) {
      try {
        parentPathFromMetadata = normalizeFilePath(file.metadata.parentPath);
      } catch {
        parentPathFromMetadata = file.metadata.parentPath;
      }
    }
    const lastSlash = cleanPath.lastIndexOf("/");
    const parentPathFromFile = lastSlash > 0 ? cleanPath.slice(0, lastSlash) : null;
    const parentPath = parentPathFromMetadata || parentPathFromFile;

    if ((location.pathname === "/fs" || location.pathname.startsWith("/drive/")) && parentPath) {
      params.set("returnTo", `${location.pathname}?path=${encodeURIComponent(parentPath)}`);
    } else {
      params.set("returnTo", currentRoute);
    }

    navigate(`/studio/edit?${params.toString()}`);
  };

  const handleWatchFolder = async () => {
    try {
      const targetPath = normalizeFilePath(file.path);
      const folder = await invoke<WatchedFolder>('add_watched_folder', {
        path: targetPath,
        recursive: true,
        autoTranscribeVideos: true,
      });
      setWatchedFolder(folder);
      toast.success('Folder is now watched');
    } catch (error) {
      toast.error(`Failed to watch folder: ${error}`);
    }
  };

  const handleSetWatchEnabled = async (enabled: boolean) => {
    if (!watchedFolder) return;
    try {
      await invoke('set_watched_folder_enabled', {
        folderId: watchedFolder.id,
        enabled,
      });
      setWatchedFolder((prev) => (prev ? { ...prev, enabled } : prev));
      toast.success(enabled ? 'Watching resumed' : 'Watching paused');
    } catch (error) {
      toast.error(`Failed to update watch state: ${error}`);
    }
  };

  const handleScanWatchedFolder = async () => {
    if (!watchedFolder) return;
    try {
      await invoke('trigger_watched_folder_backfill', { folderId: watchedFolder.id });
      toast.success('Watched folder scan started');
    } catch (error) {
      toast.error(`Failed to scan watched folder: ${error}`);
    }
  };

  const handleStopWatchingFolder = async () => {
    if (!watchedFolder) return;
    try {
      await invoke('remove_watched_folder', { folderId: watchedFolder.id });
      setWatchedFolder(null);
      toast.success('Stopped watching folder');
    } catch (error) {
      toast.error(`Failed to stop watching folder: ${error}`);
    }
  };

  const canTranscribe = isTranscribableFile(file.name) && !!onTranscribeFile;
  const canViewTranscription = isTranscribableFile(file.name) && !!onViewTranscription;
  const canAddToIndex =
    !isAlreadyIndexed &&
    !isIndexing &&
    file.type !== 'directory' &&
    !!onAddToIndex &&
    file.type !== 'audio';

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        {children}
      </ContextMenuTrigger>
      <ContextMenuContent className="w-56">
        {/* Quick Actions */}
        <ContextMenuItem onClick={handleOpenWithDefault}>
          <ExternalLink className="h-4 w-4 mr-2" />
          Open with Default App
        </ContextMenuItem>

        <ContextMenuSeparator />

        {/* Copy Actions */}
        <ContextMenuItem onClick={handleCopyPath}>
          <Copy className="h-4 w-4 mr-2" />
          Copy File Path
        </ContextMenuItem>

        <ContextMenuSeparator />

        {canTranscribe && (
          <ContextMenuItem onClick={handleTranscribe} disabled={isTranscribing}>
            <BookOpen className="h-4 w-4 mr-2" />
            {isTranscribing ? 'Transcribing...' : 'Transcribe'}
          </ContextMenuItem>
        )}

        {/* View Transcription for transcribable files */}
        {canViewTranscription && (
          <ContextMenuItem onClick={handleViewTranscription}>
            <BookOpen className="h-4 w-4 mr-2" />
            View Transcription
          </ContextMenuItem>
        )}

        {(canTranscribe || canViewTranscription) && <ContextMenuSeparator />}

        <ContextMenuItem onClick={handleSendToStudio}>
          <Video className="h-4 w-4 mr-2" />
          Send to Studio
        </ContextMenuItem>
        <ContextMenuSeparator />

        {/* System Actions */}
        <ContextMenuItem onClick={handleRevealInFinder}>
          <FolderOpen className="h-4 w-4 mr-2" />
          {navigator.platform.includes('Mac') ? 'Reveal in Finder' : 'Show in Explorer'}
        </ContextMenuItem>

        {file.type === 'directory' && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem
              onClick={handleWatchFolder}
              disabled={isWatchStateLoading || !!watchedFolder}
            >
              <FolderSearch className="h-4 w-4 mr-2" />
              Watch Folder
            </ContextMenuItem>
            {watchedFolder && (
              <>
                <ContextMenuItem onClick={() => handleSetWatchEnabled(!watchedFolder.enabled)}>
                  <FolderSearch className="h-4 w-4 mr-2" />
                  {watchedFolder.enabled ? 'Pause Watching' : 'Resume Watching'}
                </ContextMenuItem>
                <ContextMenuItem onClick={handleScanWatchedFolder}>
                  <FolderSearch className="h-4 w-4 mr-2" />
                  Scan Watched Folder Now
                </ContextMenuItem>
                <ContextMenuItem onClick={handleStopWatchingFolder}>
                  <Trash2 className="h-4 w-4 mr-2" />
                  Stop Watching Folder
                </ContextMenuItem>
              </>
            )}
          </>
        )}

        {/* Indexing and Transcription Actions - Only show if not already indexed */}
        {canAddToIndex && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem onClick={() => onAddToIndex?.(file.path)}>
              <FileText className="h-4 w-4 mr-2" />
              Add to Search Index
            </ContextMenuItem>
          </>
        )}

        {isIndexing && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem disabled>
              <FileText className="h-4 w-4 mr-2" />
              Indexing...
            </ContextMenuItem>
          </>
        )}

        {/* Info about indexed status */}
        {isAlreadyIndexed && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem disabled>
              <Info className="h-4 w-4 mr-2 dark:text-customGreen text-green-600" />
              Already in Search Index
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
    </ContextMenu>
  );
}
