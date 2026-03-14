import React, { useState, useEffect, useRef } from 'react';
import { MediaFile } from './types';
import { 
  ChevronDown, 
  ChevronUp, 
  Video, 
  Image, 
  Music, 
  File,
  Folder,
  Loader2,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { PreviewContextMenu } from './PreviewContextMenu';

interface ListViewProps {
  files: MediaFile[];
  onFileSelect: (file: MediaFile) => void;
  indexingPaths?: Set<string>;
  onAddToIndex?: (path: string) => void;
  onTranscribeFile?: (path: string) => void;
  isIndexingDisabled?: boolean;
  onLoadMore?: () => void;
  hasMoreFiles?: boolean;
  isLoadingMore?: boolean;
}

type SortField = 'name' | 'type' | 'size' | 'modified';
type SortDirection = 'asc' | 'desc';

interface Column {
  key: SortField;
  label: string;
  width?: string;
  sortable?: boolean;
}

export function ListView({ 
  files, 
  onFileSelect, 
  indexingPaths,
  onAddToIndex,
  onTranscribeFile,
  isIndexingDisabled,
  onLoadMore,
  hasMoreFiles,
  isLoadingMore,
}: ListViewProps) {
  const [sortField, setSortField] = useState<SortField>('name');
  const [sortDirection, setSortDirection] = useState<SortDirection>('asc');



  const columns: Column[] = [
    { key: 'name', label: 'Name', width: '70%', sortable: true },
    { key: 'type', label: 'Type', width: '30%', sortable: true },
  ];

  const formatFileSize = (bytes: number): string => {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  const getFileIcon = (type: string, file: MediaFile) => {
    if (file.metadata.isVideoFrame) {
      return <Video className="h-4 w-4 text-blue-500" />;
    }
    switch (type) {
      case 'video':
        return <Video className="h-4 w-4 dark:text-customBlue text-blue-500" />;
      case 'image':
        return <Image className="h-4 w-4 dark:text-customYellow text-green-500" />;
      case 'audio':
        return <Music className="h-4 w-4 dark:text-customPurple text-purple-500" />;
      case 'directory':
        return <Folder className="h-4 w-4 dark:text-customBlue text-blue-500" />;
      default:
        return <File className="h-4 w-4 dark:text-customGray text-gray-500" />;
    }
  };

  const getMatchLabel = (file: MediaFile): string | null => {
    if (file.metadata.sourceType === 'transcript_chunk') {
      return file.metadata.timestampFormatted
        ? `Transcript @ ${file.metadata.timestampFormatted}`
        : 'Transcript match';
    }

    if (file.metadata.isVideoFrame) {
      return file.metadata.timestampFormatted
        ? `Frame @ ${file.metadata.timestampFormatted}`
        : 'Frame match';
    }

    if (
      file.metadata.sourceType === 'text_chunk' ||
      file.metadata.sourceType === 'text_document'
    ) {
      return 'Text match';
    }

    return null;
  };

  const getSnippetPreview = (file: MediaFile): string | null => {
    const snippet = file.metadata.snippet;
    if (!snippet) return null;
    const normalized = snippet.replace(/\s+/g, ' ').trim();
    return normalized.length > 220 ? `${normalized.slice(0, 220)}...` : normalized;
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };

  const sortedFiles = [...files].sort((a, b) => {
    const modifier = sortDirection === 'asc' ? 1 : -1;
    
    switch (sortField) {
      case 'name':
        return a.name.localeCompare(b.name) * modifier;
      case 'type':
        return a.type.localeCompare(b.type) * modifier;
      case 'size':
        return ((a.metadata?.size || 0) - (b.metadata?.size || 0)) * modifier;
      case 'modified':
        return (
          (new Date(a.metadata?.modified || 0).getTime() -
            new Date(b.metadata?.modified || 0).getTime()) * modifier
        );
      default:
        return 0;
    }
  });

  const renderSortIcon = (field: SortField) => {
    if (sortField !== field) return null;
    return sortDirection === 'asc' ? (
      <ChevronUp className="h-4 w-4" />
    ) : (
      <ChevronDown className="h-4 w-4" />
    );
  };

  return (
    <div className="h-full overflow-auto">
      <table className="w-full border-collapse">
        <thead className="dark:bg-darkBg bg-gray-50 sticky top-0">
          <tr>
            {columns.map((column) => (
              <th
                key={column.key}
                className="text-left px-4 py-3 text-sm font-medium dark:text-customGray text-gray-500 border-b dark:border-darkBgHighlight"
                style={{ width: column.width }}
              >
                {column.sortable ? (
                  <button
                    className="flex items-center gap-1 dark:hover:text-customBlue hover:text-gray-700"
                    onClick={() => handleSort(column.key)}
                  >
                    {column.label}
                    {renderSortIcon(column.key)}
                  </button>
                ) : (
                  column.label
                )}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="dark:bg-darkBg bg-white">
          {sortedFiles.map((file, index) => (
            <PreviewContextMenu
              key={`${file.path}-${file.type}-${file.metadata.isVideoFrame ? 'frame' : 'file'}-${file.metadata.frameNumber || ''}-${index}`}
              file={file}
              onAddToIndex={onAddToIndex}
              onTranscribeFile={onTranscribeFile}
              indexingPaths={indexingPaths}
              isFromSearch={file.metadata.score !== undefined && file.metadata.score > 0}
            >
              <tr
                className="dark:hover:bg-darkBgHighlight hover:bg-gray-50 cursor-pointer"
                onClick={() => onFileSelect(file)}
              >
                <td className="px-4 py-3 border-b dark:border-darkBgHighlight">
                  <div className="flex items-center gap-2">
                    {getFileIcon(file.type, file)}
                    <div className="min-w-0 flex-1">
                      <div className="truncate">{file.name}</div>
                      {getMatchLabel(file) && (
                        <div className="text-xs text-blue-600 dark:text-blue-300 truncate">
                          {getMatchLabel(file)}
                        </div>
                      )}
                      {getSnippetPreview(file) && (
                        <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
                          {getSnippetPreview(file)}
                        </div>
                      )}
                    </div>
                    {indexingPaths?.has(file.path) && (
                      <span className="text-xs text-blue-500">Indexing...</span>
                    )}
                  </div>
                </td>
                <td className="px-4 py-3 border-b dark:border-darkBgHighlight text-sm dark:text-customGray text-gray-500">
                  {file.metadata.isVideoFrame ? 'Video' : file.type.charAt(0).toUpperCase() + file.type.slice(1)}
                </td>
              </tr>
            </PreviewContextMenu>
          ))}
        </tbody>
      </table>

      {hasMoreFiles && onLoadMore && (
        <div className="flex justify-center py-4">
          <button
            type="button"
            onClick={onLoadMore}
            disabled={isLoadingMore}
            className="inline-flex items-center gap-2 rounded-md border border-gray-300 dark:border-darkBgHighlight px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-darkBgHighlight disabled:opacity-60 disabled:cursor-not-allowed"
          >
            {isLoadingMore ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading...
              </>
            ) : (
              "Load More"
            )}
          </button>
        </div>
      )}


      {/* End of results indicator */}
      {!hasMoreFiles && files.length > 0 && (
        <div className="flex justify-center py-8">
          <span className="dark:text-customGray text-gray-500 text-sm">
            {files.length === 1 ? '1 file' : `${files.length} files`} loaded
          </span>
        </div>
      )}
    </div>
  );
} 
