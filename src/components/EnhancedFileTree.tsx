import { useState, useCallback, useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import { ChevronRight, ChevronDown, Folder, File, Eye, EyeOff, Loader2 } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import { FileItem } from './FileTree'
import { loadDirectoryWithPermissions } from '../utils/permissionManager'
import { cn } from '../lib/utils'
import { FileContextMenu } from './ContextMenu'

interface EnhancedFileTreeProps {
  items: FileItem[];
  onSelect: (file: FileItem) => void;
  onCreateFolder?: (parent: FileItem) => void;
  onDelete?: (file: FileItem) => void;
  onAddToFavorites?: (file: FileItem) => void;
  onShare?: (file: FileItem) => void;
  indexingPaths?: Set<string>;
  isDrillDown?: boolean;
  onBulkIndex?: (item: FileItem) => void;
  onTranscribeFile?: (item: FileItem) => void;
  onTranscribeDirectory?: (item: FileItem) => void;
  isIndexingDisabled?: boolean;
  onNavigateToDirectory?: (directory: FileItem) => void;
  isParentExpanded?: boolean;
}

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
  return path.replace(/\/+$/, '').toLowerCase();
};

function FileTreeItem({
  item,
  level = 0,
  isExpanded,
  children,
  indexingPaths,
  onSelect,
  onNavigateToDirectory,
  onCreateFolder,
  onDelete,
  onAddToFavorites,
  onShare,
  onBulkIndex,
  isIndexingDisabled,
  onToggleExpand,
  isParentExpanded = true,
  expandedItems,
  loadedChildren,
  watchedFoldersByPath,
}: {
  item: FileItem;
  level?: number;
  isExpanded: boolean;
  children: FileItem[];
  indexingPaths?: Set<string>;
  onSelect: (file: FileItem) => void;
  onNavigateToDirectory?: (directory: FileItem) => void;
  onCreateFolder?: (parent: FileItem) => void;
  onDelete?: (file: FileItem) => void;
  onAddToFavorites?: (file: FileItem) => void;
  onShare?: (file: FileItem) => void;
  onBulkIndex?: (item: FileItem) => void;
  onTranscribeFile?: (item: FileItem) => void;
  onTranscribeDirectory?: (item: FileItem) => void;
  isIndexingDisabled?: boolean;
  onToggleExpand: (item: FileItem) => void;
  isParentExpanded?: boolean;
  expandedItems: Set<string>;
  loadedChildren: Record<string, FileItem[]>;
  watchedFoldersByPath: Record<string, WatchedFolder>;
}) {
  const [searchParams] = useSearchParams();
  const currentPath = searchParams.get("path") || "";
  const isSelected = currentPath === item.path;
  const isIndexing = indexingPaths?.has(item.path);
  const watchedFolder =
    item.is_dir ? watchedFoldersByPath[normalizeWatchedPath(item.path)] : undefined;

  const handleItemClick = () => {
    onSelect(item);
  };

  const handleItemDoubleClick = () => {
    if (item.is_dir && onNavigateToDirectory) {
      onNavigateToDirectory(item);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Stop propagation to prevent sidebar navigation
    e.stopPropagation();
    
    switch (e.key) {
      case 'Enter':
        e.preventDefault();
        if (item.is_dir && onNavigateToDirectory) {
          onNavigateToDirectory(item);
        } else {
          onSelect(item);
        }
        break;
      case ' ':
        e.preventDefault();
        onSelect(item);
        break;
      case 'ArrowRight':
        if (item.is_dir && !isExpanded) {
          e.preventDefault();
          onToggleExpand(item);
        }
        break;
      case 'ArrowLeft':
        if (item.is_dir && isExpanded) {
          e.preventDefault();
          onToggleExpand(item);
        }
        break;
      case 'Escape':
        // Allow users to escape from file tree back to main navigation
        e.preventDefault();
        const navButton = (e.target as HTMLElement).closest('[data-nav-button]');
        (navButton as HTMLElement)?.focus();
        break;
    }
  };

  return (
    <div key={item.path}>
      <FileContextMenu
        item={item}
        onAddToFavorites={onAddToFavorites}
        onShare={onShare}
        onDelete={onDelete}
        onCreateFolder={onCreateFolder}
        onBulkIndex={onBulkIndex}
        isIndexingDisabled={isIndexingDisabled}
      >
        <div
          className={cn(
            "flex items-center py-1 px-2 rounded-md cursor-pointer",
            "focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-darkBg",
            isSelected ? "bg-blue-50 dark:bg-blueShadow" : "dark:hover:bg-darkBgHighlight hover:bg-bgShadow",
            level > 0 && "ml-4"
          )}
          onClick={handleItemClick}
          onDoubleClick={handleItemDoubleClick}
          onKeyDown={handleKeyDown}
          tabIndex={isParentExpanded ? 0 : -1}
          role="treeitem"
          aria-selected={isSelected}
          aria-expanded={item.is_dir ? isExpanded : undefined}
          aria-label={`${item.is_dir ? 'Folder' : 'File'}: ${item.name}`}
        >
          {item.is_dir ? (
            <button
              className="mr-1 dark:hover:bg-customBlue hover:bg-gray-100 rounded p-0.5 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1"
              onClick={(e) => {
                e.stopPropagation();
                onToggleExpand(item);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  e.stopPropagation();
                  onToggleExpand(item);
                }
              }}
              tabIndex={-1}
              aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${item.name}`}
            >
              {isExpanded ? (
                <ChevronDown className="h-4 w-4 text-customGray" />
              ) : (
                <ChevronRight className="h-4 w-4 text-customGray" />
              )}
            </button>
          ) : (
            <div className="w-6" />
          )}
          <div className="flex items-center gap-2 flex-1 min-w-0">
            {item.is_dir ? (
              <Folder className="h-4 w-4 text-customGray flex-shrink-0" />
            ) : (
              <File className="h-4 w-4 text-customGray flex-shrink-0" />
            )}
            <span className="text-sm truncate flex-1 min-w-0">{item.name}</span>
            {watchedFolder && (
              <span
                className="flex-shrink-0"
                title={
                  watchedFolder.status === 'scanning'
                    ? 'Watched folder (scanning)'
                    : watchedFolder.enabled
                      ? 'Watched folder'
                      : 'Watched folder (paused)'
                }
              >
                {watchedFolder.status === 'scanning' ? (
                  <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin" />
                ) : watchedFolder.enabled ? (
                  <Eye className="h-3.5 w-3.5 text-emerald-500" />
                ) : (
                  <EyeOff className="h-3.5 w-3.5 text-gray-400" />
                )}
              </span>
            )}
            {isIndexing && (
              <span className="text-xs text-blue-500 flex-shrink-0">Indexing...</span>
            )}
          </div>
        </div>
      </FileContextMenu>

      {isExpanded && item.is_dir && (
        <div>
          {children.map(child => (
            <FileTreeItem
              key={child.path}
              item={child}
              level={level + 1}
              isExpanded={expandedItems.has(child.path)}
              children={loadedChildren[child.path] || []}
              indexingPaths={indexingPaths}
              onSelect={onSelect}
              onNavigateToDirectory={onNavigateToDirectory}
              onCreateFolder={onCreateFolder}
              onDelete={onDelete}
              onAddToFavorites={onAddToFavorites}
              onShare={onShare}
              onBulkIndex={onBulkIndex}
              isIndexingDisabled={isIndexingDisabled}
              onToggleExpand={onToggleExpand}
              isParentExpanded={isParentExpanded}
              expandedItems={expandedItems}
              loadedChildren={loadedChildren}
              watchedFoldersByPath={watchedFoldersByPath}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function EnhancedFileTree({
  items: initialItems,
  onSelect,
  onCreateFolder,
  onDelete,
  onAddToFavorites,
  onShare,
  indexingPaths,
  onBulkIndex,
  onTranscribeFile,
  onTranscribeDirectory,
  isIndexingDisabled,
  onNavigateToDirectory,
  isParentExpanded = true,
}: EnhancedFileTreeProps) {
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set())
  const [loadedChildren, setLoadedChildren] = useState<Record<string, FileItem[]>>({})
  const [watchedFoldersByPath, setWatchedFoldersByPath] = useState<Record<string, WatchedFolder>>(
    {}
  );

  const loadWatchedFolders = useCallback(async () => {
    try {
      const folders = await invoke<WatchedFolder[]>('list_watched_folders');
      const nextMap: Record<string, WatchedFolder> = {};
      folders.forEach((folder) => {
        nextMap[normalizeWatchedPath(folder.path)] = folder;
      });
      setWatchedFoldersByPath(nextMap);
    } catch {
      // Keep file tree resilient if watched-folder command is unavailable.
      setWatchedFoldersByPath({});
    }
  }, []);

  useEffect(() => {
    loadWatchedFolders();
  }, [loadWatchedFolders]);

  useEffect(() => {
    let unlistenUpdated: (() => void) | null = null;
    let unlistenProgress: (() => void) | null = null;

    const setup = async () => {
      unlistenUpdated = await listen('watched_folder_updated', () => {
        loadWatchedFolders();
      });
      unlistenProgress = await listen('watched_folder_scan_progress', () => {
        loadWatchedFolders();
      });
    };

    setup();

    return () => {
      if (unlistenUpdated) unlistenUpdated();
      if (unlistenProgress) unlistenProgress();
    };
  }, [loadWatchedFolders]);

  const toggleExpand = useCallback(async (item: FileItem) => {
    if (!item.is_dir) return

    setExpandedItems(prev => {
      const next = new Set(prev)
      if (next.has(item.path)) {
        next.delete(item.path)
      } else {
        next.add(item.path)
        // Load children if not already loaded
        if (!loadedChildren[item.path]) {
          loadChildren(item.path)
        }
      }
      return next
    })
  }, [loadedChildren])

  const loadChildren = async (path: string) => {
    try {
      const children = await loadDirectoryWithPermissions(path)
      setLoadedChildren(prev => ({
        ...prev,
        [path]: children
      }))
    } catch (error) {
      console.error('Failed to load children:', error)
      setLoadedChildren(prev => ({
        ...prev,
        [path]: []
      }))
    }
  }

  return (
    <div 
      className="py-2" 
      data-tour="file-tree"
      role="tree"
      aria-label="File system tree"
    >
      {initialItems.map(item => (
        <FileTreeItem
          key={item.path}
          item={item}
          level={0}
          isExpanded={expandedItems.has(item.path)}
          children={loadedChildren[item.path] || []}
          indexingPaths={indexingPaths}
          onSelect={onSelect}
          onNavigateToDirectory={onNavigateToDirectory}
          onCreateFolder={onCreateFolder}
          onDelete={onDelete}
          onAddToFavorites={onAddToFavorites}
          onShare={onShare}
          onBulkIndex={onBulkIndex}
          onTranscribeFile={onTranscribeFile}
          onTranscribeDirectory={onTranscribeDirectory}
          isIndexingDisabled={isIndexingDisabled}
          onToggleExpand={toggleExpand}
          isParentExpanded={isParentExpanded}
          expandedItems={expandedItems}
          loadedChildren={loadedChildren}
          watchedFoldersByPath={watchedFoldersByPath}
        />
      ))}
    </div>
  )
} 
