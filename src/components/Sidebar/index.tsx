import React, { useState, useCallback, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { cn } from "../../lib/utils";
import {
  ChevronLeft,
  ChevronRight,
  Brain,
  House,
  HardDrive,
  Loader2,
  AlertCircle,
  Palette,
} from "lucide-react";
import { FileItem } from "../FileTree";
import { homeDir } from "@tauri-apps/api/path";
import { useAppLayout } from "../../contexts/AppLayoutContext";
import { NavButton } from "./NavButton";
import { FileTree } from "./FileTree";

// Import DriveInfo type from context
type DriveInfo = {
  uuid: string;
  name: string;
  custom_name?: string | null;
  physical_location?: string | null;
  mount_path: string;
  status: 'connected' | 'disconnected' | 'indexing' | 'error';
  indexed_files_count: number;
  is_removable: boolean;
};

interface SidebarProps {
  onAddToFavorites?: (file: FileItem) => void;
  onShare?: (file: FileItem) => void;
  onDelete?: (file: FileItem) => void;
  onCreateFolder?: (parent: FileItem) => void;
  indexingPaths?: Set<string>;
  onBulkIndex?: (item: FileItem) => void;
  isIndexingDisabled?: boolean;
  onClearSearch?: () => void;
}

export function Sidebar({
  onAddToFavorites,
  onShare,
  onDelete,
  onCreateFolder,
  indexingPaths,
  onBulkIndex,
  isIndexingDisabled,
  onClearSearch,
}: SidebarProps) {
  const navigate = useNavigate();
  const { clearSearch, indexedCount, drives, isDrivesLoading } = useAppLayout();

  const [isCollapsed, setIsCollapsed] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(256);
  const [isResizing, setIsResizing] = useState(false);
  const [initialPath, setInitialPath] = useState<string>("");

  useEffect(() => {
    const initPath = async () => {
      try {
        const homePath = await homeDir();
        setInitialPath(homePath);
      } catch (error) {
        console.error("Failed to get home directory:", error);
        setInitialPath("/");
      }
    };
    initPath();
  }, []);

  const handleNavigateToDirectory = useCallback(
    (directory: FileItem) => {
      if (!directory.is_dir) return;
      onClearSearch?.();
      navigate(`/fs?path=${encodeURIComponent(directory.path)}`);
    },
    [navigate, onClearSearch]
  );

  const handleFileSelect = useCallback(
    (file: FileItem) => {
      onClearSearch?.();
      navigate(`/fs?path=${encodeURIComponent(file.path)}`);
    },
    [navigate, onClearSearch]
  );

  const handleDriveSelect = useCallback((drive: DriveInfo | null) => {
    onClearSearch?.();

    if (!drive) {
      navigate("/fs");
      return;
    }

    if (drive.status === 'disconnected') {
      console.log(`Drive "${drive.name}" is disconnected. Cannot browse offline drives.`);
      return;
    }

    if (!drive.mount_path || drive.mount_path.trim() === '') {
      console.log(`Drive "${drive.name}" has no valid mount path. It may not be mounted.`);
      navigate(`/drive/${drive.uuid}`);
      return;
    }

    navigate(`/drive/${drive.uuid}?path=${encodeURIComponent(drive.mount_path)}`);
  }, [navigate, onClearSearch]);

  const getDriveIcon = useCallback((drive: DriveInfo) => {
    switch (drive.status) {
      case 'connected':
        return <HardDrive className="text-blue-400 font-bold w-5 h-5" />;
      case 'disconnected':
        return <HardDrive className="text-gray-400 font-bold w-5 h-5" />;
      case 'indexing':
        return <Loader2 className="text-blue-500 font-bold w-5 h-5 animate-spin" />;
      case 'error':
        return <AlertCircle className="text-red-500 font-bold w-5 h-5" />;
      default:
        return <HardDrive className="text-gray-500 font-bold w-5 h-5" />;
    }
  }, []);

  // Resize handlers
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isResizing) return;
      const newWidth = Math.min(Math.max(e.clientX, 200), 500);
      setSidebarWidth(newWidth);
    },
    [isResizing]
  );

  const handleMouseUp = useCallback(() => {
    setIsResizing(false);
  }, []);

  useEffect(() => {
    if (isResizing) {
      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";

      return () => {
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      };
    }
  }, [isResizing, handleMouseMove, handleMouseUp]);

  const basePath = initialPath;
  const userNamePath = basePath.split("/")[2];



  const handleSidebarKeyDown = useCallback((e: React.KeyboardEvent) => {
    // Only handle navigation when not focused on file tree items
    if ((e.target as HTMLElement).closest('[role="tree"]')) {
      return;
    }

    const navButtons = Array.from((e.currentTarget as HTMLElement).querySelectorAll('[data-nav-button]'));
    const currentIndex = navButtons.findIndex(btn => btn === document.activeElement);

    switch (e.key) {
      case 'ArrowDown':
      case 'j': // Vim-style navigation
        e.preventDefault();
        const nextIndex = currentIndex < navButtons.length - 1 ? currentIndex + 1 : 0;
        (navButtons[nextIndex] as HTMLElement)?.focus();
        break;
      case 'ArrowUp':
      case 'k': // Vim-style navigation
        e.preventDefault();
        const prevIndex = currentIndex > 0 ? currentIndex - 1 : navButtons.length - 1;
        (navButtons[prevIndex] as HTMLElement)?.focus();
        break;
      case 'Home':
        e.preventDefault();
        (navButtons[0] as HTMLElement)?.focus();
        break;
      case 'End':
        e.preventDefault();
        const lastIndex = navButtons.length - 1;
        (navButtons[lastIndex] as HTMLElement)?.focus();
        break;
    }
  }, []);

  return (
    <aside
      className={cn(
        "h-full min-h-0 overflow-hidden flex flex-col dark:bg-darkBg border-r dark:border-darkBgHighlight border-gray transition-all duration-300 relative",
        isCollapsed ? "w-12 delay-50" : "w-64 delay-0"
      )}
      style={{ width: isCollapsed ? undefined : `${sidebarWidth}px` }}
      aria-label="Sidebar navigation"
      role="complementary"
    >
      {/* Collapse toggle button */}
      <button
        onClick={() => setIsCollapsed(!isCollapsed)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setIsCollapsed(!isCollapsed);
          }
        }}
        className="absolute right-0 top-4 dark:bg-darkBgHighlight bg-white border-gray dark:hover:bg-customGray hover:bg-customBlue rounded-r-lg p-1.5 shadow-sm hover:shadow transition-all z-10 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-darkBg"
        aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        aria-expanded={!isCollapsed}
        tabIndex={0}
      >
        {isCollapsed ? (
          <ChevronRight className="h-4 w-4" />
        ) : (
          <ChevronLeft className="h-4 w-4" />
        )}
      </button>

      <div className={
        cn(
          "w-full h-full min-h-0 flex flex-col transition-opacity duration-150",
          isCollapsed ? "opacity-0 delay-0" : "opacity-100 delay-150",
        )
      }>
        {/* Resize handle */}
        {!isCollapsed && (
          <div
            className="absolute right-0 top-0 w-1 h-full cursor-col-resize hover:bg-blue-500 transition-colors z-20"
            onMouseDown={handleMouseDown}
          />
        )}

        {/* Navigation */}
        <div
          className={cn(
            "flex-1 min-h-0 overflow-y-auto overscroll-contain py-4",
            isCollapsed && "overflow-hidden"
          )}
          onKeyDown={handleSidebarKeyDown}
          aria-label="Use arrow keys or j/k to navigate between sections"
        >

          <NavButton
            to="/"
            onClick={() => {
              clearSearch();
              // Dispatch event for tour
              window.dispatchEvent(
                new CustomEvent("ai-library-clicked", {
                  detail: { collection: "indexed" },
                })
              );
            }}>
            <NavButton.Trigger data-tour="ai-library-button">
              <NavButton.Icon className="bg-yellow-600/20 rounded-full h-fit w-fit p-2" icon={<Brain className="text-yellow-500 font-bold w-5 h-5" />} />
              <NavButton.Label label="AI Library" description={`${indexedCount.toLocaleString()} ${indexedCount === 1 ? 'item' : 'items'}`} />
            </NavButton.Trigger>
          </NavButton>

          <div className="border-t border-gray-200 dark:border-gray-700"></div>

          <NavButton
            isExpandable
            onClick={() => {
              // Dispatch event for tour when home nav is expanded
              setTimeout(() => {
                window.dispatchEvent(
                  new CustomEvent("home-nav-expanded", {
                    detail: { expanded: true },
                  })
                );
              }, 100); // Small delay to ensure expansion state is updated
            }}
          >
            <NavButton.Trigger data-tour="home-nav-button">
              <NavButton.Icon className="bg-blue-600/20 rounded-full h-fit w-fit p-2" icon={<House className="text-blue-400 font-bold w-5 h-5" />} />
              <NavButton.Label label={userNamePath} />
            </NavButton.Trigger>
            <NavButton.ExtendedContent>
              <FileTree
                basePath={basePath}
                onNavigateToDirectory={handleNavigateToDirectory}
                onSelect={handleFileSelect}
                onCreateFolder={onCreateFolder}
                onDelete={onDelete}
                onAddToFavorites={onAddToFavorites}
                onShare={onShare}
                indexingPaths={indexingPaths}
                onBulkIndex={onBulkIndex}
                isIndexingDisabled={isIndexingDisabled}
                isCollapsed={isCollapsed}
              />
            </NavButton.ExtendedContent>
          </NavButton>

          <div className="border-t border-gray-200 dark:border-gray-700"></div>

          {isDrivesLoading ? (
            <div className="flex items-center space-x-2 px-6 py-3 text-sm text-gray-500 dark:text-gray-400" role="status" aria-live="polite">
              <Loader2 className="w-4 h-4 animate-spin" />
              <span>Loading drives...</span>
            </div>
          ) : drives.length === 0 ? null : (
            <div role="group" aria-label="Connected drives">
              {drives.map((drive, index) => {
                const isOnline = drive.status === 'connected' || drive.status === 'indexing';
                const driveName = drive.custom_name || drive.name;
                const Separator = index !== drives.length - 1 ? <div className="border-t border-gray-200 dark:border-gray-700"></div> : null;

                return isOnline ? (
                  <React.Fragment key={drive.uuid}>
                    <NavButton
                      isExpandable={isOnline}
                      onClick={isOnline ? undefined : () => handleDriveSelect(drive)}
                    >
                      <NavButton.Trigger>
                        <NavButton.Icon
                          className="bg-blue-600/20 rounded-full h-fit w-fit p-2"
                          icon={getDriveIcon(drive)}
                        />
                        <NavButton.Label
                          label={driveName}
                          description={drive.status === 'disconnected' ? 'offline' : `${drive.indexed_files_count} items`}
                        />
                      </NavButton.Trigger>
                      {isOnline && (
                        <NavButton.ExtendedContent>
                          <FileTree
                            basePath={drive.mount_path}
                            onNavigateToDirectory={handleNavigateToDirectory}
                            onSelect={handleFileSelect}
                            onCreateFolder={onCreateFolder}
                            onDelete={onDelete}
                            onAddToFavorites={onAddToFavorites}
                            onShare={onShare}
                            indexingPaths={indexingPaths}
                            onBulkIndex={onBulkIndex}
                            isIndexingDisabled={isIndexingDisabled}
                            isCollapsed={isCollapsed}
                          />
                        </NavButton.ExtendedContent>
                      )}
                    </NavButton>
                    {Separator}
                  </React.Fragment>
                ) : null;
              })}
            </div>
          )}


          <NavButton
            to="/studio"
            onClick={() => {
              clearSearch();
            }}>
            <NavButton.Trigger>
              <NavButton.Icon className="bg-purple-600/20 rounded-full h-fit w-fit p-2" icon={<Palette className="text-purple-500 font-bold w-5 h-5" />} />
              <NavButton.Label label="Studio" description="Create & Edit" />
            </NavButton.Trigger>
          </NavButton>
        </div>
      </div>
    </aside>
  );
}
