import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface FileCluster {
  cluster_id: number;
  name: string;
  position_x: number;
  position_y: number;
  dominant_type: string;
  auto_tags: string[];
  file_count: number;
}

export interface FilePosition2D {
  file_id: string;
  file_path: string;
  x: number;
  y: number;
  cluster_id: number;
  source_type: string;
}

export interface UseClustersReturn {
  clusters: FileCluster[];
  positions: FilePosition2D[];
  isLoading: boolean;
  error: string | null;
  recompute: () => Promise<void>;
  loadExisting: () => Promise<void>;
}

export function useClusters(): UseClustersReturn {
  const [clusters, setClusters] = useState<FileCluster[]>([]);
  const [positions, setPositions] = useState<FilePosition2D[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isMounted = useRef(true);

  const loadExisting = useCallback(async () => {
    try {
      const [c, p] = await Promise.all([
        invoke<FileCluster[]>("get_clusters"),
        invoke<FilePosition2D[]>("get_file_positions"),
      ]);
      if (!isMounted.current) return;
      setClusters(c);
      setPositions(p);
      setError(null);
    } catch (e) {
      if (!isMounted.current) return;
      // No clusters computed yet is not an error
      setClusters([]);
      setPositions([]);
    }
  }, []);

  const recompute = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const newClusters = await invoke<FileCluster[]>("compute_clusters");
      if (!isMounted.current) return;
      setClusters(newClusters);
      const newPositions = await invoke<FilePosition2D[]>("get_file_positions");
      if (!isMounted.current) return;
      setPositions(newPositions);
    } catch (e) {
      if (!isMounted.current) return;
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (isMounted.current) setIsLoading(false);
    }
  }, []);

  // Load existing clusters on mount
  useEffect(() => {
    isMounted.current = true;
    loadExisting();
    return () => {
      isMounted.current = false;
    };
  }, [loadExisting]);

  // Auto-recompute when indexing jobs complete
  useEffect(() => {
    let unlistenJob: (() => void) | null = null;
    let unlistenBulk: (() => void) | null = null;
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    const scheduleRecompute = () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        recompute();
      }, 2000);
    };

    const setup = async () => {
      unlistenJob = await listen("job_completed", scheduleRecompute);
      unlistenBulk = await listen("bulk_index_progress", (event: any) => {
        // Only recompute when bulk indexing finishes
        if (event?.payload?.status === "completed") {
          scheduleRecompute();
        }
      });
    };

    setup();

    return () => {
      if (unlistenJob) unlistenJob();
      if (unlistenBulk) unlistenBulk();
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  }, [recompute]);

  return { clusters, positions, isLoading, error, recompute, loadExisting };
}
