import { useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { ViewMode, MediaFile } from './types';
import { Button } from '../ui/button';
import { List, LayoutGrid, FolderSearch } from 'lucide-react';
import { GridView } from './GridView';
import { ListView } from './ListView';

import { invoke } from '@tauri-apps/api/core';
import { resolve } from '@tauri-apps/api/path';
import { ReferenceImageData } from '../SearchBar';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

interface PreviewContainerProps {
  files: MediaFile[];
  initialViewMode?: ViewMode;
  onLoadMore?: () => void;
  isLoadingFiles?: boolean;
  onDirectorySelect?: (directory: MediaFile) => void;
  indexingPaths?: Set<string>;
  transcribingPaths?: Set<string>;
  onToggleWatch?: (path: string) => void;
  onAddToIndex?: (path: string) => void;
  onTranscribeFile?: (path: string) => void;
  onBulkIndex?: (path: string) => void;
  currentDirectoryPath?: string;
  isIndexingDisabled?: boolean;
  hasMoreFiles?: boolean;
  isLoadingMore?: boolean;
  referenceImage?: ReferenceImageData | null;
  onReferenceImageClose?: () => void;
  isSearchMode?: boolean;
  selectedCollection?: string;
  totalCount?: number;
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  fileTypeFilter: string;
  setFileTypeFilter: (filter: string) => void;
}

export function PreviewActions({
  viewMode,
  setViewMode,
  fileTypeFilter,
  setFileTypeFilter,
  onBulkIndex,
  currentDirectoryPath,
  isIndexingDisabled,
  handleBulkIndexClick
}: {
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  fileTypeFilter: string;
  setFileTypeFilter: (filter: string) => void;
  onBulkIndex?: (path: string) => void;
  currentDirectoryPath?: string;
  isIndexingDisabled?: boolean;
  handleBulkIndexClick: () => void;
}) {
  return (
    <div className="flex items-center gap-3 flex-wrap">
      <Button
        variant={viewMode === 'grid' ? 'default' : 'ghost'}
        size="sm"
        onClick={() => setViewMode('grid')}
        className={`h-9 px-4 font-medium ${viewMode === 'grid' ? 'dark:hover:bg-blueShadow dark:bg-darkBgHighlight bg-gray-100' : 'dark:text-customGray text-gray-500 hover:text-blueShadow'}`}
      >
        <LayoutGrid className="h-4 w-4 mr-2" />
        Grid
      </Button>
      <Button
        variant={viewMode === 'list' ? 'default' : 'ghost'}
        size="sm"
        onClick={() => setViewMode('list')}
        className={`h-9 px-4 font-medium ${viewMode === 'list' ? 'dark:hover:bg-blueShadow dark:bg-darkBgHighlight bg-gray-100' : 'dark:text-customGray text-gray-500 hover:text-blueShadow'}`}
      >
        <List className="h-4 w-4 mr-2" />
        List
      </Button>
      <Select value={fileTypeFilter} onValueChange={setFileTypeFilter}>
        <SelectTrigger className="w-[120px] h-9 dark:bg-darkBg dark:border-darkBgHighlight bg-white border-gray-200 shadow-sm dark:hover:bg-darkBg hover:bg-white dark:focus:bg-darkBg focus:bg-white">
          <SelectValue placeholder="File Type" />
        </SelectTrigger>
        <SelectContent className="dark:bg-darkBg dark:border-darkBgHighlight bg-white">
          <SelectItem value="all">All Files</SelectItem>
          <SelectItem value="image">Images</SelectItem>
          <SelectItem value="video">Videos</SelectItem>
        </SelectContent>
      </Select>
      {currentDirectoryPath && onBulkIndex && (
        <Button
          variant="outline"
          size="sm"
          onClick={handleBulkIndexClick}
          disabled={isIndexingDisabled}
          className="h-9 px-4 font-medium"
        >
          <FolderSearch className="h-4 w-4 mr-2" />
          Index All Files
        </Button>
      )}
    </div>
  );
}

export function PreviewContainer({
  files,
  onLoadMore,
  isLoadingFiles,
  onDirectorySelect,
  indexingPaths,
  onToggleWatch,
  onAddToIndex,
  onTranscribeFile,
  onBulkIndex,
  currentDirectoryPath,
  isIndexingDisabled,
  hasMoreFiles,
  isLoadingMore,
  isSearchMode = false,
  selectedCollection,
  totalCount,
  viewMode,
  setViewMode,
  fileTypeFilter,
}: PreviewContainerProps) {
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement>(null);

  const filteredFiles = files.filter(file => {
    if (fileTypeFilter === 'all') return true;
    if (fileTypeFilter === 'image') return file.type === 'image' && !file.metadata.isVideoFrame;
    if (fileTypeFilter === 'video') return file.type === 'video' || file.metadata.isVideoFrame;
    return true;
  });

  const handleFileSelect = (file: MediaFile) => {
    if (file.type === 'directory' && onDirectorySelect) {
      onDirectorySelect(file);
    } else {
      const timestamp = file.metadata?.timestamp;
      const url = timestamp
        ? `/studio/edit?path=${file.path}&timestamp=${timestamp}`
        : `/studio/edit?path=${file.path}`;
      navigate(url);
    }
  };

  const handleAddToIndex = async (path: string) => {
    try {
      // Clean up the path by removing the asset://localhost prefix and decoding the URL
      const cleanPath = decodeURIComponent(path.replace('asset://localhost/', ''));

      // Use Tauri's path.resolve to get the absolute path 
      const absolutePath = await resolve(cleanPath);

      // Check if the file exists before trying to index it
      const exists = await invoke<boolean>('file_exists', { path: absolutePath });

      if (!exists) {
        console.error('File does not exist at path:', absolutePath);
        return;
      }

      // Call the parent onAddToIndex callback
      if (onAddToIndex) {
        await onAddToIndex(absolutePath);
      }
    } catch (error) {
      console.error('Failed to index file:', error);
    }
  };

  const handleBulkIndexClick = async () => {
    if (currentDirectoryPath && onBulkIndex) {
      // Clean up the path if it has the asset:// prefix
      const cleanPath = currentDirectoryPath.replace('asset://localhost/', '');
      const absolutePath = await resolve(cleanPath);
      onBulkIndex(absolutePath);
    }
  };

  // Add file count display
  // For indexed collection, use totalCount from backend; otherwise use actual file count
  const totalFiles = typeof totalCount === 'number' && totalCount > 0 ? totalCount : files.length;
  const shownFiles = filteredFiles.length;

  return (
    <div ref={containerRef} className="flex flex-col dark:bg-darkBgMid bg-gray-50 h-full max-h-[calc(100vh-110px)]">
      <div className="px-6 py-3 text-sm dark:text-customGray text-gray-500">
        Showing {shownFiles} of {totalFiles} files
      </div>
      <div className="overflow-auto h-full">
        {viewMode === 'grid' && (
          <GridView
            key={filteredFiles.length > 0 ? `${filteredFiles[0]?.path}-${filteredFiles.length}` : 'empty'}
            files={filteredFiles}
            onFileSelect={handleFileSelect}
            isLoading={isLoadingFiles}
            indexingPaths={indexingPaths}
            onToggleWatch={onToggleWatch}
            onAddToIndex={handleAddToIndex}
            onTranscribeFile={onTranscribeFile}
            isIndexingDisabled={isIndexingDisabled}
            onLoadMore={onLoadMore}
            hasMoreFiles={hasMoreFiles}
            isLoadingMore={isLoadingMore}
          />
        )}
        {viewMode === 'list' && (
          <ListView
            files={filteredFiles}
            onFileSelect={handleFileSelect}
            indexingPaths={indexingPaths}
            onAddToIndex={handleAddToIndex}
            onTranscribeFile={onTranscribeFile}
            isIndexingDisabled={isIndexingDisabled}
            onLoadMore={onLoadMore}
            hasMoreFiles={hasMoreFiles}
            isLoadingMore={isLoadingMore}
          />
        )}

      </div>
    </div>
  );
}
