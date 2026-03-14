import { useSearchParams, useMatch } from "react-router-dom";
import { useState, useCallback, useEffect } from "react";

import { EnhancedFileTree } from "../EnhancedFileTree";
import { FileItem } from "../FileTree";
import { loadDirectoryWithPermissions } from "../../utils/permissionManager";
import { useNavButton } from "./NavButton";

export function FileTree({
    basePath,
    onNavigateToDirectory,
    onSelect,
    onCreateFolder,
    onDelete,
    onAddToFavorites,
    onShare,
    indexingPaths,
    onBulkIndex,
    isIndexingDisabled,
    isCollapsed,
}: {
    basePath: string;
    onNavigateToDirectory: (directory: FileItem) => void;
    onSelect: (file: FileItem) => void;
    onCreateFolder?: (parent: FileItem) => void;
    onDelete?: (file: FileItem) => void;
    onAddToFavorites?: (file: FileItem) => void;
    onShare?: (file: FileItem) => void;
    indexingPaths?: Set<string>;
    onBulkIndex?: (item: FileItem) => void;
    isIndexingDisabled?: boolean;
    isCollapsed: boolean;
}) {
    const [items, setItems] = useState<FileItem[]>([]);
    const [searchParams] = useSearchParams();
    const driveMatch = useMatch("/drive/:drive_id");
    const { expanded } = useNavButton();

    const loadItems = useCallback(async (path: string) => {
        if (!path) return;
        try {
            const files = await loadDirectoryWithPermissions(path);
            setItems(files);
        } catch (error) {
            console.error("Failed to load directory:", error);
            setItems([]);
        }
    }, []);

    // Load items based on current route and path
    useEffect(() => {
        if (driveMatch) {
            // In drive mode: load current drive path
            const currentPath = searchParams.get("path");
            if (currentPath) {
                loadItems(currentPath);
            } else {
                setItems([]); // No path in drive mode = empty
            }
        } else {
            // In file system mode: load base path (home directory)
            if (basePath) {
                loadItems(basePath);
            }
        }
    }, [basePath, driveMatch, searchParams, loadItems]);

    if (isCollapsed) return null;

    return (
        <div className="px-3 transition-opacity duration-300 opacity-100">
            {basePath && (
                <EnhancedFileTree
                    items={items}
                    onSelect={onSelect}
                    onCreateFolder={onCreateFolder}
                    onDelete={onDelete}
                    onAddToFavorites={onAddToFavorites}
                    onShare={onShare}
                    indexingPaths={indexingPaths}
                    onBulkIndex={onBulkIndex}
                    isIndexingDisabled={isIndexingDisabled}
                    onNavigateToDirectory={onNavigateToDirectory}
                    isParentExpanded={expanded}
                />
            )}
        </div>
    );
}
