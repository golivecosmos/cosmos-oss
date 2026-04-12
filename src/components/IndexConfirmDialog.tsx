import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, FolderOpen, Check } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Button } from "./ui/button";

interface SubdirInfo {
  name: string;
  path: string;
  file_count: number;
  total_size: number;
}

interface ScanResult {
  file_count: number;
  dir_count: number;
  total_size_bytes: number;
  top_subdirs: SubdirInfo[];
}

interface IndexConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  directoryPath: string;
  onConfirm: (paths: string[]) => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function estimateTime(fileCount: number): string {
  // ~20 files/sec for images, slower for video/text. Rough estimate.
  const seconds = Math.ceil(fileCount / 15);
  if (seconds < 60) return `~${seconds}s`;
  if (seconds < 3600) return `~${Math.ceil(seconds / 60)} min`;
  return `~${(seconds / 3600).toFixed(1)} hr`;
}

export const IndexConfirmDialog: React.FC<IndexConfirmDialogProps> = ({
  open,
  onOpenChange,
  directoryPath,
  onConfirm,
}) => {
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());

  const dirName = directoryPath.split("/").pop() || directoryPath;

  // Scan when dialog opens
  useEffect(() => {
    if (!open || !directoryPath) return;
    setScanResult(null);
    setError(null);
    setIsScanning(true);
    setSelectedPaths(new Set());

    invoke<ScanResult>("scan_directory", { path: directoryPath })
      .then((result) => {
        setScanResult(result);
        // Select all subdirs by default
        const allPaths = new Set(result.top_subdirs.map((s) => s.path));
        setSelectedPaths(allPaths);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setIsScanning(false));
  }, [open, directoryPath]);

  const toggleSubdir = (path: string) => {
    setSelectedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const toggleAll = () => {
    if (!scanResult) return;
    const allPaths = scanResult.top_subdirs.map((s) => s.path);
    if (selectedPaths.size === allPaths.length) {
      setSelectedPaths(new Set());
    } else {
      setSelectedPaths(new Set(allPaths));
    }
  };

  const selectedFileCount = scanResult
    ? scanResult.top_subdirs
        .filter((s) => selectedPaths.has(s.path))
        .reduce((sum, s) => sum + s.file_count, 0)
    : 0;

  // Files directly in the root (not in any subdir)
  const rootFileCount = scanResult
    ? scanResult.file_count -
      scanResult.top_subdirs.reduce((sum, s) => sum + s.file_count, 0)
    : 0;

  const handleConfirm = () => {
    if (!scanResult) return;
    if (scanResult.top_subdirs.length === 0 || selectedPaths.size === scanResult.top_subdirs.length) {
      // All selected or no subdirs — index the whole directory
      onConfirm([directoryPath]);
    } else {
      // Index only selected subdirectories + root files
      const paths = Array.from(selectedPaths);
      if (rootFileCount > 0) {
        // Root-level files still need the parent path
        // The backend will handle dedup via the indexed_paths set
        paths.unshift(directoryPath);
      }
      onConfirm(paths);
    }
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FolderOpen className="w-5 h-5" />
            Index "{dirName}"
          </DialogTitle>
          <DialogDescription>
            Review what will be indexed before starting.
          </DialogDescription>
        </DialogHeader>

        {isScanning && (
          <div className="flex items-center justify-center py-8 gap-2 text-muted-foreground">
            <Loader2 className="w-4 h-4 animate-spin" />
            Scanning directory...
          </div>
        )}

        {error && (
          <div className="py-4 text-sm text-destructive">
            Failed to scan: {error}
          </div>
        )}

        {scanResult && (
          <div className="space-y-4">
            {/* Summary stats */}
            <div className="flex items-center justify-between text-sm">
              <span>
                <span className="font-medium">{scanResult.file_count.toLocaleString()}</span> files
                {" · "}
                <span className="text-muted-foreground">{formatBytes(scanResult.total_size_bytes)}</span>
              </span>
              <span className="text-muted-foreground">
                {estimateTime(scanResult.file_count)}
              </span>
            </div>

            {/* Subdirectory list */}
            {scanResult.top_subdirs.length > 0 && (
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Subdirectories
                  </span>
                  <button
                    onClick={toggleAll}
                    className="text-xs text-primary hover:underline"
                  >
                    {selectedPaths.size === scanResult.top_subdirs.length
                      ? "Deselect all"
                      : "Select all"}
                  </button>
                </div>

                <div className="max-h-[240px] overflow-y-auto rounded-md border">
                  {scanResult.top_subdirs.map((subdir) => (
                    <button
                      key={subdir.path}
                      onClick={() => toggleSubdir(subdir.path)}
                      className="flex items-center gap-2 w-full px-3 py-1.5 text-sm hover:bg-accent transition-colors text-left"
                    >
                      <div
                        className={`w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors ${
                          selectedPaths.has(subdir.path)
                            ? "bg-primary border-primary"
                            : "border-input"
                        }`}
                      >
                        {selectedPaths.has(subdir.path) && (
                          <Check className="w-3 h-3 text-primary-foreground" />
                        )}
                      </div>
                      <span className="truncate flex-1">{subdir.name}</span>
                      <span className="text-xs text-muted-foreground whitespace-nowrap">
                        {subdir.file_count} files · {formatBytes(subdir.total_size)}
                      </span>
                    </button>
                  ))}
                </div>

                {rootFileCount > 0 && (
                  <p className="text-xs text-muted-foreground">
                    + {rootFileCount} files in root directory
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isScanning || !scanResult || (scanResult.top_subdirs.length > 0 && selectedPaths.size === 0 && rootFileCount === 0)}
          >
            {scanResult
              ? `Index ${selectedPaths.size === scanResult.top_subdirs.length ? "all" : selectedFileCount.toLocaleString()} files`
              : "Index"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
