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
  Video
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { normalizeFilePath } from '../../lib/utils';
import { toast } from 'sonner';
import { useNavigate } from 'react-router-dom';

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
  const [isVideoInStudio, setIsVideoInStudio] = useState(false);
  const [isCheckingStudio, setIsCheckingStudio] = useState(false);

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

  const handleSendToStudio = async () => {
    const cleanPath = normalizeFilePath(file.path);
    navigate(`/studio/edit?path=${cleanPath}`);
  };
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

        {/* View Transcription for transcribable files */}
        {isTranscribableFile(file.name) && onViewTranscription && (
          <ContextMenuItem onClick={handleViewTranscription}>
            <BookOpen className="h-4 w-4 mr-2" />
            View Transcription
          </ContextMenuItem>
        )}

        {isTranscribableFile(file.name) && onViewTranscription && <ContextMenuSeparator />}

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

        {/* Indexing and Transcription Actions - Only show if not already indexed */}
        {!isAlreadyIndexed && !isIndexing && file.type !== 'directory' && (
          <>
            <ContextMenuSeparator />
            {/* Regular indexing option */}
            {onAddToIndex && (
              <ContextMenuItem onClick={() => onAddToIndex(file.path)}>
                <FileText className="h-4 w-4 mr-2" />
                Add to Search Index
              </ContextMenuItem>
            )}

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
