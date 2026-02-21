import React from "react";
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
} from "lucide-react";
import { FileItem } from "./FileTree";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

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
  const handleIndexFile = async () => {
    try {
      toast.success("Added file to search index queue");
      await invoke("index_file", {
        path: item.path,
        name: item.name,
        isDirectory: item.is_dir,
      });
    } catch (error) {
      console.error("Failed to index file:", error);
      toast.error("Failed to index file");
    }
  };


  return (
    <BaseContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        {!item.is_dir && (
          <>
            {/* Regular indexing option */}
            {isIndexingDisabled ? (
              <ContextMenuItem disabled>
                <Database className="mr-2 h-4 w-4" />
                Add to Index
              </ContextMenuItem>
            ) : (
              <ContextMenuItem onClick={handleIndexFile}>
                <Database className="mr-2 h-4 w-4" />
                Add to Index
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
