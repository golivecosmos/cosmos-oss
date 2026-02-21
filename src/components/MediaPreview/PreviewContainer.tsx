import { useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { ViewMode, MediaFile } from './types';
import { Button } from '../ui/button';
import { List, LayoutGrid, FolderSearch } from 'lucide-react';
import { GridView } from './GridView';
import { ListView } from './ListView';

import { invoke } from '@tauri-apps/api/tauri';
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
  const [expandedDir, setExpandedDir] = useState<string | null>(null);

  const filteredFiles = files.filter(file => {
    if (fileTypeFilter === 'all') return true;
    if (fileTypeFilter === 'image') return file.type === 'image' && !file.metadata.isVideoFrame;
    if (fileTypeFilter === 'video') return file.type === 'video' || file.metadata.isVideoFrame;
    return true;
  });

  // Group files by parent directory when viewing AI Library (not search results)
  let groupedFiles: Record<string, MediaFile[]> = {};
  let shouldGroupByDirectory = !isSearchMode && selectedCollection === 'indexed' && !currentDirectoryPath && !isLoadingFiles && !isLoadingMore && filteredFiles.length > 0;

  if (shouldGroupByDirectory) {
    filteredFiles.forEach(file => {
      const dir = file.metadata.parentPath || 'Unknown';
      if (!groupedFiles[dir]) groupedFiles[dir] = [];
      groupedFiles[dir].push(file);
    });
  }

  // Step 1: Prepare folderTiles for the folder tile grid
  let folderTiles: { dir: string; files: MediaFile[]; previewFile?: MediaFile }[] = [];
  if (shouldGroupByDirectory) {
    folderTiles = Object.entries(groupedFiles).map(([dir, files]) => ({
      dir,
      files,
      previewFile: files[0],
    }));
  }

  // Pagination for folder tiles
  const FOLDER_TILE_PAGE_SIZE = 500; // Show many more compact folders
  const [folderTilePage, setFolderTilePage] = useState(100); // Start with more folders loaded
  const paginatedFolderTiles = folderTiles.slice(0, folderTilePage * FOLDER_TILE_PAGE_SIZE);
  const hasMoreFolderTiles = folderTiles.length > paginatedFolderTiles.length;

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

  const handleTileClick = (dir: string) => {
    setExpandedDir(prev => (prev === dir ? null : dir));
  };

  // Add file count display
  // For indexed collection, use totalCount from backend; otherwise use actual file count
  const totalFiles = typeof totalCount === 'number' && totalCount > 0 ? totalCount : files.length;
  const shownFiles = filteredFiles.length;

  return (
    <div ref={containerRef} className="flex flex-col dark:bg-darkBgMid bg-gray-50 h-full max-h-[calc(100vh-110px)]">
      {/* File count display */}
      <div className="px-6 py-2 text-sm dark:text-customGray text-gray-500">Showing {shownFiles} of {totalFiles} files</div>
      <div className="overflow-auto h-full">
        {shouldGroupByDirectory ? (
          <div className="p-6">
            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10 gap-8">
              {paginatedFolderTiles.map(({ dir, files, previewFile }) => {
                const isExpanded = expandedDir === dir;
                const folderName = dir.split('/').pop() || 'Folder';
                const fileCount = files.length;

                return (
                  <div
                    key={dir}
                    className={
                      "group flex flex-col items-center cursor-pointer transition-all duration-150" +
                      (isExpanded ? " opacity-100" : " opacity-90 hover:opacity-100")
                    }
                    onClick={() => handleTileClick(dir)}
                    style={{ width: '120px' }}
                  >
                    {/* Folder icon - larger, Finder-style */}
                    <div className={
                      "relative mb-1 p-2 rounded-lg transition-all" +
                      (isExpanded ? " bg-blue-500/20 dark:bg-blueHighlight/20" : " hover:bg-gray-100/10 dark:hover:bg-white/5")
                    }>
                      <svg width="64" height="64" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M8 20C8 17.7909 9.79086 16 12 16H24L28 20H52C54.2091 20 56 21.7909 56 24V48C56 50.2091 54.2091 52 52 52H12C9.79086 52 8 50.2091 8 48V20Z" 
                              className="fill-blue-500 dark:fill-blueHighlight"/>
                        <path d="M8 18C8 15.7909 9.79086 14 12 14H22L26 18H52C54.2091 18 56 19.7909 56 22V24H8V18Z" 
                              className="fill-blue-600 dark:fill-blue-400"/>
                      </svg>
                    </div>
                    
                    {/* Folder name */}
                    <div className="text-center w-full px-1">
                      <p className="text-xs dark:text-gray-200 text-gray-700 font-normal truncate">
                        {folderName}
                      </p>
                      <p className="text-[10px] dark:text-gray-400 text-gray-500 mt-0.5">
                        {fileCount} {fileCount === 1 ? 'item' : 'items'}
                      </p>
                    </div>
                  </div>
                );
              })}

            </div>
            {hasMoreFolderTiles && (
              <div className="mt-6 text-center">
                <button
                  className="py-2 px-4 dark:bg-darkBgHighlight dark:hover:bg-blueShadow bg-gray-100 rounded-lg dark:text-text text-gray-700 text-sm font-medium hover:bg-gray-200 transition"
                  onClick={e => { e.preventDefault(); setFolderTilePage(p => p + 1); }}
                >
                  Load More Folders
                </button>
              </div>
            )}
            {/* Divider and expanded section header */}
            {expandedDir && (
              <>
                <div className="my-6 border-t dark:border-darkBgHighlight border-gray-200 w-full" />
                <div className="flex items-center gap-2 mb-4 px-2">
                  <FolderSearch className="h-5 w-5 dark:text-blueHighlight text-blue-500" />
                  <span className="font-semibold text-lg dark:text-text text-gray-800">
                    {expandedDir.split('/').pop()}
                  </span>
                  <span className="text-sm dark:text-customGray text-gray-500">
                    ({groupedFiles[expandedDir]?.length || 0} files)
                  </span>
                  <button
                    className="ml-auto px-3 py-1 rounded dark:bg-darkBgHighlight bg-gray-100 dark:text-text text-gray-600 dark:hover:bg-blueShadow hover:bg-gray-200 text-sm font-medium"
                    onClick={() => setExpandedDir(null)}
                  >
                    Close
                  </button>
                </div>
              </>
            )}
            {/* Expanded directory group below the grid */}
            {expandedDir && (
              <div className="mt-4">
                {folderTiles.filter(({ dir }) => dir === expandedDir).map(({ dir, files }) => (
                  <div key={dir} className="dark:bg-darkBg bg-white rounded-lg border dark:border-darkBgHighlight border-gray-200">
                    <div className="p-4">
                      {viewMode === 'grid' ? (
                        <GridView
                          key={files.length > 0 ? `${files[0]?.path}-${files.length}` : 'empty'}
                          files={files}
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
                      ) : (
                        <ListView
                          files={files}
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
                ))}
              </div>
            )}
          </div>
        ) : (
          <>
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
          </>
        )}

      </div>
    </div>
  );
}
